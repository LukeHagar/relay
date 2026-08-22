//! Capture plane (§3.3): webhook ingestion with byte-faithful capture and
//! channel-configurable responses. No rewrites, no normalization.

use crate::envelope::{now_utc, CaptureInput};
use crate::store::{slug_of_host, Channel, ResponseMode};
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use std::net::SocketAddr;
use std::sync::Arc;

pub fn router(state: crate::AppState) -> axum::Router {
    axum::Router::new().fallback(capture_any).with_state(state)
}

pub(crate) fn api_error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        axum::Json(serde_json::json!({"error": {"code": code, "message": message.into()}})),
    )
        .into_response()
}

/// Resolve `(channel name, faithful event path)` per §3.1 addressing.
pub(crate) fn resolve_target(
    state: &crate::AppState,
    host: Option<&str>,
    raw_path: &str,
) -> Option<(String, String)> {
    // Hosted mode wins: one hostname per channel; path is the full wire path.
    if let (Some(zone), Some(host)) = (&state.config.capture_zone, host) {
        if let Some(slug) = slug_of_host(host, zone) {
            if state.store.get(&slug).is_some() {
                return Some((slug, raw_path.to_string()));
            }
        }
    }
    // Local addressing: `/c/{name}` root is structural; suffix is user space.
    let rest = raw_path.strip_prefix("/c/")?;
    let (name, suffix) = match rest.split_once('/') {
        Some((n, s)) => (n, format!("/{s}")),
        None => (rest, String::new()),
    };
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), suffix))
}

async fn capture_any(State(state): State<crate::AppState>, req: Request) -> Response {
    let method = req.method().as_str().to_ascii_uppercase();
    let host = req
        .headers()
        .get(http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| req.uri().host().map(|h| h.to_string()));
    // http::Uri preserves the wire encoding of path and query (§1 fidelity guarantee).
    let raw_path = req.uri().path().to_string();
    let raw_query = req.uri().query().map(|s| s.to_string());
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect();
    let remote_addr: Option<String> = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.to_string());

    let Some((channel_name, event_path)) = resolve_target(&state, host.as_deref(), &raw_path)
    else {
        return api_error(
            StatusCode::NOT_FOUND,
            "no_such_channel",
            "unknown capture target",
        );
    };
    let Some(channel) = state.store.get(&channel_name) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "no_such_channel",
            format!("channel '{channel_name}' does not exist"),
        );
    };

    if let Some(err) = check_auth(&channel, &headers) {
        return err;
    }
    if !channel.rate_limiter().allow() {
        return api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "channel capture rate limit exceeded",
        );
    }

    let cfg = channel.config();
    let (body_bytes, truncated) = read_limited(req.into_body(), state.config.max_body_bytes).await;

    let input = CaptureInput {
        channel: channel.name.clone(),
        host,
        method,
        raw_path: event_path,
        raw_query,
        headers,
        body: body_bytes.clone(),
        truncated,
        status_sent: planned_status(&cfg),
        remote_addr,
    };
    let event = channel.push(input, now_utc());
    let mut persisted = true;
    if let Some(db) = &state.db {
        if let Err(e) = db.insert_event(&event) {
            persisted = false;
            state
                .persisted_failures
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tracing::error!(error = %e, event_id = %event.id, "failed to persist captured event");
        }
    }
    state.hub.publish(&event);
    crate::forwarding::apply(&state, &channel, &event).await;

    let mut response = respond(&cfg, &event, &body_bytes);
    if !persisted {
        // Durable-layer outage surfaced honestly without failing the capture: the
        // event is in memory and live subscribers already have it.
        response
            .headers_mut()
            .insert("x-relay-persisted", http::HeaderValue::from_static("false"));
    }
    response
}

fn planned_status(cfg: &crate::store::ChannelConfig) -> u16 {
    match cfg.response_mode() {
        ResponseMode::Ack | ResponseMode::Echo => 200,
        ResponseMode::Static => cfg
            .static_response
            .as_ref()
            .map(|s| s.status)
            .unwrap_or(200),
    }
}

fn check_auth(channel: &Arc<Channel>, headers: &[(String, String)]) -> Option<Response> {
    let auth = channel.config().auth.clone()?;
    let expected = std::env::var(&auth.bearer_env).unwrap_or_default();
    let ok = !expected.is_empty()
        && headers
            .iter()
            .any(|(k, v)| k == "authorization" && v.as_str() == format!("Bearer {expected}"));
    if ok {
        None
    } else {
        Some(api_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "capture bearer token missing or wrong",
        ))
    }
}

/// Read at most `max` bytes within a bounded time window; anything beyond `max` sets
/// `truncated`, and a client that dribbles past the window is cut off (slowloris guard).
async fn read_limited(body: Body, max: usize) -> (bytes::Bytes, bool) {
    use http_body_util::BodyExt;
    const READ_WINDOW: std::time::Duration = std::time::Duration::from_secs(20);
    let mut body = body;
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let frame = match tokio::time::timeout(READ_WINDOW, body.frame()).await {
            Err(_) => return (buf.into(), true), // window expired mid-body
            Ok(v) => v,
        };
        match frame {
            None => break,
            Some(Err(_)) => break,
            Some(Ok(frame)) => {
                if let Ok(data) = frame.into_data() {
                    let remaining = max.saturating_sub(buf.len());
                    if data.len() <= remaining {
                        buf.extend_from_slice(&data);
                    } else {
                        buf.extend_from_slice(&data[..remaining]);
                        return (buf.into(), true);
                    }
                }
            }
        }
    }
    (buf.into(), false)
}

fn respond(
    cfg: &crate::store::ChannelConfig,
    event: &crate::envelope::Envelope,
    body: &[u8],
) -> Response {
    match cfg.response_mode() {
        ResponseMode::Ack => axum::Json(serde_json::json!({
            "accepted": true,
            "event_id": event.id,
        }))
        .into_response(),
        ResponseMode::Echo => {
            let content_type = event
                .headers
                .iter()
                .find(|(k, _)| k == "content-type")
                .map(|(_, v)| v.clone());
            let mut resp = (StatusCode::OK, body.to_vec()).into_response();
            if let Some(ct) = content_type {
                if let Ok(val) = http::HeaderValue::from_str(&ct) {
                    resp.headers_mut().insert(http::header::CONTENT_TYPE, val);
                }
            }
            resp
        }
        ResponseMode::Static => {
            let Some(sr) = &cfg.static_response else {
                return api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "bad_request",
                    "static mode without static_response",
                );
            };
            let status = StatusCode::from_u16(sr.status).unwrap_or(StatusCode::OK);
            let mut resp = (status, sr.body.clone()).into_response();
            for (k, v) in &sr.headers {
                if let (Ok(name), Ok(val)) = (
                    http::HeaderName::from_bytes(k.as_bytes()),
                    http::HeaderValue::from_str(v),
                ) {
                    resp.headers_mut().insert(name, val);
                }
            }
            resp
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::store::{AuthConfig, ChannelConfig, StaticResponse, Store};
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    fn state() -> crate::AppState {
        crate::AppState {
            config: Config::default(),
            store: Arc::new(Store::default()),
            hub: Arc::new(crate::hub::Hub::default()),
            profiles: Arc::new(crate::signing::Profiles::default()),
            templates: Arc::new(
                crate::templates::Registry::load(crate::BUNDLED_TEMPLATES, None).unwrap(),
            ),
            http: reqwest::Client::new(),
            db: None,
            billing: Arc::new(crate::billing::Billing::default()),
            persisted_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    async fn send(
        router: axum::Router,
        req: HttpRequest<axum::body::Body>,
    ) -> axum::response::Response {
        router.oneshot(req).await.unwrap()
    }

    fn request(
        method: &str,
        uri: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> HttpRequest<axum::body::Body> {
        let mut b = HttpRequest::builder().method(method).uri(uri);
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        b.body(axum::body::Body::from(body.to_string())).unwrap()
    }

    #[tokio::test]
    async fn captures_with_fidelity_and_acks() {
        let st = state();
        st.store.upsert("gh", ChannelConfig::default()).unwrap();
        let router = router(st.clone());

        let resp = send(
            router,
            request(
                "POST",
                "/c/gh/push/a%2Fb?b=2&a=1",
                &[
                    ("content-type", "application/json"),
                    ("x-dup", "1"),
                    ("x-dup", "2"),
                ],
                "{\"hello\":true}",
            ),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let ack: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(ack["accepted"], true);

        let events = st.store.get("gh").unwrap().history(None, 10);
        let env = &events[0];
        assert_eq!(env.path, "/push/a%2Fb"); // percent-encoding untouched
        assert_eq!(env.query.as_deref(), Some("b=2&a=1")); // original order
        assert_eq!(env.headers.iter().filter(|(k, _)| k == "x-dup").count(), 2);
        assert_eq!(env.body, "{\"hello\":true}");
        assert_eq!(env.status_sent, Some(200));
        assert_eq!(env.method, "POST");
    }

    #[tokio::test]
    async fn bare_channel_root_captures_with_empty_path() {
        let st = state();
        st.store.upsert("root", ChannelConfig::default()).unwrap();
        let resp = send(router(st.clone()), request("GET", "/c/root", &[], "")).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let env = st
            .store
            .get("root")
            .unwrap()
            .history(None, 1)
            .pop()
            .unwrap();
        assert_eq!(env.path, "");
        assert_eq!(env.method, "GET");
    }

    #[tokio::test]
    async fn unknown_channel_is_json_404_and_not_captured() {
        let st = state();
        let resp = send(router(st.clone()), request("POST", "/c/ghost", &[], "")).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(err["error"]["code"], "no_such_channel");
    }

    #[tokio::test]
    async fn echo_mode_mirrors_slack_challenge() {
        let st = state();
        st.store
            .upsert(
                "slack",
                ChannelConfig {
                    response_mode: Some(ResponseMode::Echo),
                    ..Default::default()
                },
            )
            .unwrap();
        let body = r#"{"type":"url_verification","challenge":"xyz123"}"#;
        let resp = send(
            router(st.clone()),
            request(
                "POST",
                "/c/slack",
                &[("content-type", "application/json")],
                body,
            ),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(bytes, body.as_bytes()); // challenge echoed verbatim
    }

    #[tokio::test]
    async fn static_mode_returns_configured_response() {
        let st = state();
        st.store
            .upsert(
                "s",
                ChannelConfig {
                    response_mode: Some(ResponseMode::Static),
                    static_response: Some(StaticResponse {
                        status: 201,
                        headers: vec![("x-relay".into(), "static".into())],
                        body: "created".into(),
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        let resp = send(router(st.clone()), request("POST", "/c/s", &[], "x")).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(resp.headers().get("x-relay").unwrap(), "static");
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(&bytes[..], b"created");
    }

    #[tokio::test]
    async fn oversized_bodies_are_truncated_and_flagged() {
        let mut st = state();
        st.config.max_body_bytes = 8;
        st.store.upsert("big", ChannelConfig::default()).unwrap();
        let resp = send(
            router(st.clone()),
            request("POST", "/c/big", &[], "0123456789ABCDEF"),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let env = st.store.get("big").unwrap().history(None, 1).pop().unwrap();
        assert!(env.truncated);
        assert_eq!(env.body, "01234567");
    }

    #[tokio::test]
    async fn bearer_env_auth_gates_capture() {
        std::env::set_var("RELAY_TEST_CH_SECRET", "s3cret");
        let st = state();
        st.store
            .upsert(
                "locked",
                ChannelConfig {
                    auth: Some(AuthConfig {
                        bearer_env: "RELAY_TEST_CH_SECRET".into(),
                    }),
                    ..Default::default()
                },
            )
            .unwrap();

        let denied = send(router(st.clone()), request("POST", "/c/locked", &[], "")).await;
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

        let ok = send(
            router(st.clone()),
            request(
                "POST",
                "/c/locked",
                &[("authorization", "Bearer s3cret")],
                "",
            ),
        )
        .await;
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(st.store.get("locked").unwrap().history(None, 10).len(), 1);
    }

    #[tokio::test]
    async fn rate_limit_rejects_floods() {
        let st = state();
        st.store.upsert("hot", ChannelConfig::default()).unwrap();
        let router = router(st.clone());
        let mut codes = vec![];
        for _ in 0..320 {
            let resp = send(router.clone(), request("POST", "/c/hot", &[], "")).await;
            codes.push(resp.status());
        }
        assert!(codes.iter().filter(|c| **c == StatusCode::OK).count() >= 290);
        assert!(codes.contains(&StatusCode::TOO_MANY_REQUESTS));
    }

    #[tokio::test]
    async fn forwarding_rules_deliver_signed_copies() {
        use crate::store::ForwardRule;
        std::env::set_var("RELAY_TEST_FWD_SECRET", "fwd_secret");

        // receiver records what it got
        type CapturedDelivery = (http::Uri, bytes::Bytes, Option<String>);
        let seen: Arc<std::sync::Mutex<Vec<CapturedDelivery>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_for_app = seen.clone();
        let app = axum::Router::new().fallback(move |req: http::Request<axum::body::Body>| {
            let seen = seen_for_app.clone();
            async move {
                let uri = req.uri().clone();
                let (parts, body) = req.into_parts();
                let bytes = http_body_util::BodyExt::collect(body)
                    .await
                    .unwrap()
                    .to_bytes();
                let sig = parts
                    .headers
                    .get("x-shopify-hmac-sha256")
                    .map(|v| v.to_str().unwrap().to_string());
                seen.lock().unwrap().push((uri, bytes, sig));
                axum::http::StatusCode::OK
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let mut st = state();
        let mut profiles = crate::signing::Profiles::default();
        profiles.0.insert(
            "shopify-dev".into(),
            crate::signing::SigningProfile {
                scheme: crate::signing::Scheme::Shopify,
                secret_env: "RELAY_TEST_FWD_SECRET".into(),
            },
        );
        st.profiles = Arc::new(profiles);
        st.store
            .upsert(
                "shop",
                ChannelConfig {
                    forward: vec![ForwardRule {
                        url: format!("http://{addr}/forwarded"),
                        profile: Some("shopify-dev".into()),
                        headers: vec![],
                    }],
                    ..Default::default()
                },
            )
            .unwrap();

        send(
            router(st.clone()),
            request("POST", "/c/shop/orders/create", &[], "{}"),
        )
        .await;
        for _ in 0..40 {
            if !seen.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let got = seen.lock().unwrap().pop().expect("forward delivered");
        assert_eq!(got.0.path(), "/forwarded/orders/create"); // source path appended
        assert_eq!(&got.1[..], b"{}"); // exact body
                                       // signature recomputed over the delivered body must match the header
        let expected =
            crate::signing::sign(crate::signing::Scheme::Shopify, "fwd_secret", &got.1, 0)[0]
                .1
                .clone();
        assert_eq!(got.2.as_deref(), Some(expected.as_str()));
    }

    #[tokio::test]
    async fn hosted_zone_resolves_slug_and_keeps_full_path() {
        let mut st = state();
        st.config.capture_zone = Some("acme.capture.example.com".into());
        st.store.upsert("github", ChannelConfig::default()).unwrap();
        let resp = send(
            router(st.clone()),
            request(
                "POST",
                "/any/path/at/all",
                &[("host", "github.acme.capture.example.com")],
                "x",
            ),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let env = st
            .store
            .get("github")
            .unwrap()
            .history(None, 1)
            .pop()
            .unwrap();
        assert_eq!(env.path, "/any/path/at/all"); // full wire path, nothing stripped
        assert_eq!(env.host.as_deref(), Some("github.acme.capture.example.com"));
    }
}
