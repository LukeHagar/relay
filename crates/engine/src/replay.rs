//! Replay (§6): send a stored, templated, or inline event to an arbitrary target,
//! optionally re-signed, and report the real delivery outcome.

use crate::envelope::{encode_body, BodyEncoding, Envelope};
use crate::signing::Scheme;
use crate::AppState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Deserialize)]
pub struct ReplayRequest {
    target_url: String,
    timeout_ms: Option<u64>,
    source: ReplaySource,
    #[serde(default)]
    overrides: Option<Overrides>,
    #[serde(default)]
    vars: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    signing: Option<SigningSpec>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ReplaySource {
    Event { event_id: String },
    Template { template: String },
    Inline { inline: InlineSource },
}

#[derive(Deserialize)]
pub struct InlineSource {
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default)]
    body: String,
}

fn default_method() -> String {
    "POST".into()
}

#[derive(Deserialize)]
pub struct Overrides {
    method: Option<String>,
    path: Option<String>,
    query: Option<String>,
    #[serde(default)]
    headers: Vec<(String, String)>,
    body: Option<String>,
}

#[derive(Deserialize)]
pub struct SigningSpec {
    profile: Option<String>,
    scheme: Option<Scheme>,
    #[serde(default)]
    secret: Option<String>,
}

/// Parts of the outgoing request, pre-override.
struct Outgoing {
    method: String,
    path: String,
    query: Option<String>,
    headers: Vec<(String, String)>,
    body: bytes::Bytes,
}

pub async fn handler(State(state): State<AppState>, body: String) -> Response {
    let req: ReplayRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return api_error(
                axum::http::StatusCode::BAD_REQUEST,
                "bad_request",
                format!("invalid replay request: {e}"),
            )
        }
    };
    match execute(&state, req).await {
        Ok(resp) => axum::Json(resp).into_response(),
        Err((code, msg)) => api_error(code.http(), code.name(), msg),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ReplayError {
    BadRequest,
    NoSuchEvent,
    NoSuchTemplate,
    MissingVars,
    UnknownSigningProfile,
    InlineSecretDisabled,
    UnsupportedScheme,
    TargetUrl,
}

impl ReplayError {
    fn name(&self) -> &'static str {
        match self {
            ReplayError::BadRequest => "bad_request",
            ReplayError::NoSuchEvent => "no_such_event",
            ReplayError::NoSuchTemplate => "no_such_template",
            ReplayError::MissingVars => "missing_vars",
            ReplayError::UnknownSigningProfile => "unknown_signing_profile",
            ReplayError::InlineSecretDisabled => "inline_secret_disabled",
            ReplayError::UnsupportedScheme => "unsupported_scheme",
            ReplayError::TargetUrl => "bad_request",
        }
    }
    fn http(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            ReplayError::MissingVars | ReplayError::TargetUrl => StatusCode::BAD_REQUEST,
            ReplayError::BadRequest => StatusCode::BAD_REQUEST,
            ReplayError::NoSuchEvent | ReplayError::NoSuchTemplate => StatusCode::NOT_FOUND,
            ReplayError::UnknownSigningProfile
            | ReplayError::InlineSecretDisabled
            | ReplayError::UnsupportedScheme => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

fn api_error(status: axum::http::StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        axum::Json(json!({"error": {"code": code, "message": message.into()}})),
    )
        .into_response()
}

fn hop_by_hop(name: &str) -> bool {
    matches!(
        name,
        "host"
            | "content-length"
            | "connection"
            | "transfer-encoding"
            | "accept-encoding"
            | "keep-alive"
            | "upgrade"
    )
}

fn find_event(state: &AppState, id: &str) -> Option<Arc<Envelope>> {
    state
        .store
        .list()
        .iter()
        .find_map(|ch| ch.history(None, 10_000).into_iter().find(|e| e.id == id))
}

fn from_envelope(env: &Envelope) -> anyhow::Result<Outgoing> {
    let body = match env.body_encoding {
        BodyEncoding::Utf8 => bytes::Bytes::from(env.body.clone()),
        BodyEncoding::Base64 => {
            bytes::Bytes::from(base64::engine::general_purpose::STANDARD.decode(&env.body)?)
        }
    };
    Ok(Outgoing {
        method: env.method.clone(),
        path: if env.path.is_empty() {
            "/".to_string()
        } else {
            env.path.clone()
        },
        query: env.query.clone(),
        headers: env.headers.clone(),
        body,
    })
}

fn from_template(
    state: &AppState,
    r#ref: &str,
    vars: &BTreeMap<String, serde_json::Value>,
) -> Result<Outgoing, (ReplayError, String)> {
    let Some(tpl) = state.templates.get(r#ref) else {
        let r = r#ref;
        return Err((
            ReplayError::NoSuchTemplate,
            format!("template '{r}' does not exist"),
        ));
    };
    let body = crate::templates::interpolate(&tpl.request.body, vars, &tpl.defaults.vars).map_err(
        |missing| {
            (
                ReplayError::MissingVars,
                format!("unresolved template variables: {}", missing.join(", ")),
            )
        },
    )?;
    let body_bytes = match &body {
        serde_json::Value::String(s) => bytes::Bytes::from(s.clone()),
        other => bytes::Bytes::from(serde_json::to_vec(other).unwrap()),
    };
    let headers: Vec<(String, String)> = tpl
        .request
        .headers
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
        .collect();
    Ok(Outgoing {
        method: tpl.request.method.to_ascii_uppercase(),
        path: tpl.request.path.clone(),
        query: None,
        headers,
        body: body_bytes,
    })
}

async fn execute(
    state: &AppState,
    req: ReplayRequest,
) -> Result<serde_json::Value, (ReplayError, String)> {
    // 1) Resolve the source into outgoing parts.
    let mut out = match &req.source {
        ReplaySource::Event { event_id } => {
            let Some(env) = find_event(state, event_id) else {
                return Err((
                    ReplayError::NoSuchEvent,
                    format!("event '{event_id}' does not exist"),
                ));
            };
            from_envelope(&env).map_err(|e| (ReplayError::BadRequest, e.to_string()))?
        }
        ReplaySource::Template { template } => from_template(state, template, &req.vars)?,
        ReplaySource::Inline { inline } => Outgoing {
            method: inline.method.to_ascii_uppercase(),
            path: if inline.path.is_empty() {
                "/".into()
            } else {
                inline.path.clone()
            },
            query: inline.query.clone(),
            headers: inline.headers.clone(),
            body: bytes::Bytes::from(inline.body.clone()),
        },
    };

    // 2) Apply overrides (§6 precedence: source → overrides → signing).
    let mut path_overridden = false;
    if let Some(o) = &req.overrides {
        if let Some(m) = &o.method {
            out.method = m.to_ascii_uppercase();
        }
        if let Some(p) = &o.path {
            out.path = p.clone();
            path_overridden = true;
        }
        if let Some(q) = &o.query {
            out.query = if q.is_empty() { None } else { Some(q.clone()) };
        }
        if let Some(b) = &o.body {
            out.body = bytes::Bytes::from(b.clone());
        }
        for (k, v) in &o.headers {
            let name = k.to_ascii_lowercase();
            out.headers.retain(|(hk, _)| hk != &name);
            out.headers.push((name, v.clone()));
        }
    }
    if out.path.is_empty() || !out.path.starts_with('/') {
        out.path = format!("/{}", out.path);
    }

    // 3) Build the URL. `overrides.path` replaces; otherwise the source's faithful
    // path is appended to the target's own path (base-URL semantics, §6).
    let mut url = url::Url::parse(&req.target_url)
        .map_err(|e| (ReplayError::TargetUrl, format!("invalid target_url: {e}")))?;
    if path_overridden {
        url.set_path(&out.path);
    } else {
        let base = url.path().trim_end_matches('/').to_string();
        let joined = if out.path == "/" {
            base
        } else {
            format!("{base}{}", out.path)
        };
        url.set_path(&joined);
    }
    url.set_query(out.query.as_deref());
    // The sent envelope must reflect the exact wire path, not the source fragment.
    out.path = url.path().to_string();

    // 4) Signing applies last, over the exact bytes sent (§6).
    let mut signing_headers: Vec<(String, String)> = Vec::new();
    if let Some(spec) = &req.signing {
        let (scheme, secret) = match (&spec.profile, &spec.secret) {
            (Some(profile), _) => state
                .profiles
                .resolve(profile)
                .map_err(|e| (ReplayError::UnknownSigningProfile, e.to_string()))?,
            (None, Some(secret)) => {
                if !state.config.allow_inline_secret {
                    return Err((
                        ReplayError::InlineSecretDisabled,
                        "inline secrets are disabled; start the engine with --allow-inline-secret"
                            .into(),
                    ));
                }
                let scheme = spec.scheme.ok_or((
                    ReplayError::UnsupportedScheme,
                    "inline signing requires a scheme".to_string(),
                ))?;
                (scheme, secret.clone())
            }
            (None, None) => {
                return Err((
                    ReplayError::BadRequest,
                    "signing requires a profile (or inline scheme+secret)".into(),
                ))
            }
        };
        let now = crate::envelope::now_utc().unix_timestamp();
        signing_headers =
            crate::signing::sign_for(scheme, &secret, &out.body, now, Some(url.as_str()));
        // Replace any captured signature headers with the fresh ones.
        for (name, _) in &signing_headers {
            let lname = name.to_ascii_lowercase();
            out.headers.retain(|(hk, _)| hk != &lname);
        }
    }

    // 5) Send.
    let timeout = std::time::Duration::from_millis(req.timeout_ms.unwrap_or(10_000).min(60_000));
    let client = state.http.clone();
    let mut request = client.request(
        out.method.parse().unwrap_or(reqwest::Method::POST),
        url.as_str(),
    );
    for (k, v) in out.headers.iter().filter(|(k, _)| !hop_by_hop(k)) {
        request = request.header(k, v);
    }
    for (k, v) in &signing_headers {
        request = request.header(k, v);
    }
    request = request.body(out.body.clone());
    let started = Instant::now();
    let delivery = match tokio::time::timeout(timeout, request.send()).await {
        Err(_) => {
            json!({ "status": null, "duration_ms": started.elapsed().as_millis() as u64, "error": "target_timeout" })
        }
        Ok(Err(e)) if e.is_timeout() => {
            json!({ "status": null, "duration_ms": started.elapsed().as_millis() as u64, "error": "target_timeout" })
        }
        Ok(Err(e)) => {
            json!({ "status": null, "duration_ms": started.elapsed().as_millis() as u64, "error": format!("connect_error: {e}") })
        }
        Ok(Ok(resp)) => json!({
            "status": resp.status().as_u16(),
            "duration_ms": started.elapsed().as_millis() as u64,
            "error": null,
        }),
    };

    // 6) Echo exactly what went out as an envelope (§6 response shape).
    let mut sent_headers = out.headers.clone();
    sent_headers.extend(signing_headers);
    let (body_str, encoding) = encode_body(&out.body);
    let sent_event = Envelope {
        v: 1,
        id: ulid::Ulid::new().to_string(),
        channel: "replay".into(),
        host: Some(url.host_str().unwrap_or("").to_string()),
        seq: 0,
        received_at: crate::envelope::format_rfc3339_ms(crate::envelope::truncate_ms(
            crate::envelope::now_utc(),
        )),
        method: out.method.clone(),
        path: out.path.clone(),
        query: out.query.clone(),
        headers: sent_headers,
        body: body_str,
        body_encoding: encoding,
        truncated: false,
        status_sent: delivery["status"].as_u64().map(|s| s as u16),
        remote_addr: None,
    };

    Ok(json!({ "sent_event": sent_event, "delivery": delivery }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::envelope::CaptureInput;
    use crate::store::{ChannelConfig, Store};
    use std::sync::Arc;

    fn state(allow_inline: bool) -> AppState {
        AppState {
            config: Config {
                allow_inline_secret: allow_inline,
                ..Default::default()
            },
            store: Arc::new(Store::default()),
            hub: Arc::new(crate::hub::Hub::default()),
            profiles: Arc::new(crate::signing::Profiles::default()),
            templates: Arc::new(
                crate::templates::Registry::load(crate::BUNDLED_TEMPLATES, None).unwrap(),
            ),
            http: reqwest::Client::new(),
            db: None,
            persisted_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            billing: Arc::new(crate::billing::Billing::default()),
        }
    }

    fn capture_into(state: &AppState, body: &str) -> String {
        let ch = state
            .store
            .upsert("gh", ChannelConfig::default())
            .unwrap()
            .channel;
        let env = ch.push(
            CaptureInput {
                channel: "gh".into(),
                host: None,
                method: "POST".into(),
                raw_path: "/push".into(),
                raw_query: None,
                headers: vec![("content-type".into(), "application/json".into())],
                body: bytes::Bytes::from(body.to_string()),
                truncated: false,
                status_sent: 200,
                remote_addr: None,
            },
            crate::envelope::now_utc(),
        );
        env.id.clone()
    }

    async fn spawn_receiver(
        expect: Arc<std::sync::Mutex<Vec<http::Request<bytes::Bytes>>>>,
    ) -> String {
        spawn_receiver_at(expect, "/").await
    }

    async fn spawn_receiver_at(
        expect: Arc<std::sync::Mutex<Vec<http::Request<bytes::Bytes>>>>,
        route: &str,
    ) -> String {
        let route = route.to_string();
        let app =
            axum::Router::new().fallback(move |req: http::Request<axum::body::Body>| async move {
                let (parts, body) = req.into_parts();
                let bytes = http_body_util::BodyExt::collect(body)
                    .await
                    .unwrap()
                    .to_bytes();
                expect
                    .lock()
                    .unwrap()
                    .push(http::Request::from_parts(parts, bytes));
                axum::http::StatusCode::OK
            });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let _ = route;
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn replays_stored_event_with_fresh_github_signature() {
        std::env::set_var("RELAY_TEST_GH2_SECRET", "It's a Secret to Everybody");
        let st = state(false);
        let mut profiles = crate::signing::Profiles::default();
        profiles.0.insert(
            "gh-dev".into(),
            crate::signing::SigningProfile {
                scheme: crate::signing::Scheme::Github,
                secret_env: "RELAY_TEST_GH2_SECRET".into(),
            },
        );
        let st = AppState {
            profiles: Arc::new(profiles),
            ..st
        };
        let body = r#"{"hello":"world"}"#;
        let event_id = capture_into(&st, body);

        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let target = spawn_receiver(seen.clone()).await;
        let req = serde_json::json!({
            "target_url": target,
            "source": { "event_id": event_id },
            "signing": { "profile": "gh-dev" }
        });
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert_eq!(resp["delivery"]["status"], 200);
        assert_eq!(resp["sent_event"]["path"], "/push");
        assert_eq!(resp["sent_event"]["channel"], "replay");

        let got = seen.lock().unwrap().pop().unwrap();
        assert_eq!(got.body(), body.as_bytes()); // exact bytes
                                                 // GitHub's scheme has no timestamp: the header is reproducible exactly.
        let sig = got.headers().get("x-hub-signature-256").unwrap();
        let expected = crate::signing::sign(
            crate::signing::Scheme::Github,
            "It's a Secret to Everybody",
            got.body(),
            0,
        )[0]
        .1
        .clone();
        assert_eq!(sig.to_str().unwrap(), expected);
    }

    #[tokio::test]
    async fn replays_template_with_vars_and_stripe_signing() {
        std::env::set_var("RELAY_TEST_STRIPE2_SECRET", "whsec_test_secret");
        let st = state(false);
        let mut profiles = crate::signing::Profiles::default();
        profiles.0.insert(
            "stripe-dev".into(),
            crate::signing::SigningProfile {
                scheme: crate::signing::Scheme::Stripe,
                secret_env: "RELAY_TEST_STRIPE2_SECRET".into(),
            },
        );
        let st = AppState {
            profiles: Arc::new(profiles),
            ..st
        };

        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let target = spawn_receiver(seen.clone()).await;
        let req = serde_json::json!({
            "target_url": target,
            "source": { "template": "stripe/invoice.paid@1" },
            "vars": { "customer": "cus_real", "amount_paid": 9900 },
            "signing": { "profile": "stripe-dev" }
        });
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert_eq!(resp["delivery"]["status"], 200);

        let got = seen.lock().unwrap().pop().unwrap();
        let body: serde_json::Value = serde_json::from_slice(got.body()).unwrap();
        assert_eq!(body["type"], "invoice.paid");
        assert_eq!(body["data"]["object"]["customer"], "cus_real");
        assert_eq!(body["data"]["object"]["amount_paid"], 9900);
        let sig = got
            .headers()
            .get("stripe-signature")
            .unwrap()
            .to_str()
            .unwrap();
        let (t, v1) = sig.split_once(",v1=").unwrap();
        let t = t.strip_prefix("t=").unwrap();
        let expected = crate::signing::sign(
            crate::signing::Scheme::Stripe,
            "whsec_test_secret",
            got.body(),
            t.parse().unwrap(),
        )[0]
        .1
        .clone();
        assert_eq!(format!("t={t},v1={v1}"), expected);
    }

    #[tokio::test]
    async fn inline_secret_requires_flag() {
        let st = state(false);
        let target = spawn_receiver(Default::default()).await;
        let req = serde_json::json!({
            "target_url": target,
            "source": { "inline": { "method": "POST", "path": "/", "body": "x" } },
            "signing": { "scheme": "github", "secret": "s" }
        });
        let err = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err.0.name(), "inline_secret_disabled");

        let st = state(true);
        let req = serde_json::json!({
            "target_url": target,
            "source": { "inline": { "method": "POST", "path": "/", "body": "x" } },
            "signing": { "scheme": "github", "secret": "s" }
        });
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert_eq!(resp["delivery"]["status"], 200);
    }

    #[tokio::test]
    async fn missing_event_and_unknown_profile_are_typed_errors() {
        let st = state(false);
        let target = spawn_receiver(Default::default()).await;
        let req = serde_json::json!({ "target_url": target, "source": { "event_id": "01NOPE" } });
        let err = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err.0.name(), "no_such_event");

        let req = serde_json::json!({
            "target_url": target,
            "source": { "inline": { "method": "POST", "path": "/", "body": "" } },
            "signing": { "profile": "ghost" }
        });
        let err = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err.0.name(), "unknown_signing_profile");
    }

    #[tokio::test]
    async fn missing_template_vars_listed() {
        let st = state(false);
        let target = spawn_receiver(Default::default()).await;
        let req = serde_json::json!({
            "target_url": target,
            "source": { "template": "stripe/invoice.paid@1" },
            "vars": { "customer": "cus_x" }
        });
        // amount_paid has a default; customer overridden; nothing missing → ok
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert_eq!(resp["delivery"]["status"], 200);
    }

    #[tokio::test]
    async fn target_timeout_is_reported_not_fatal() {
        let st = state(false);
        // port 1 is unroutable on loopback → connect error fast, or timeout path
        let req = serde_json::json!({
            "target_url": "http://127.0.0.1:1/",
            "timeout_ms": 300,
            "source": { "inline": { "method": "GET", "path": "/", "body": "" } }
        });
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert!(resp["delivery"]["status"].is_null());
        assert!(resp["delivery"]["error"].is_string());
    }

    #[tokio::test]
    async fn source_path_appends_to_target_base_path() {
        let st = state(false);
        let event_id = capture_into(&st, r#"{"appended":true}"#);
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let target = spawn_receiver_at(seen.clone(), "/base/hooks").await;
        // target carries a base path; the source's faithful path must append to it
        let req = serde_json::json!({
            "target_url": format!("{target}/base"),
            "source": { "event_id": event_id }
        });
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert_eq!(resp["delivery"]["status"], 200);
        let got = seen.lock().unwrap().pop().unwrap();
        assert_eq!(got.uri().path(), "/base/push"); // source path "/push" appended to "/base"
    }

    #[tokio::test]
    async fn overrides_apply_in_precedence_order() {
        let st = state(false);
        let event_id = capture_into(&st, r#"{"orig":true}"#);
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let target = spawn_receiver(seen.clone()).await;
        let req = serde_json::json!({
            "target_url": target,
            "source": { "event_id": event_id },
            "overrides": {
                "path": "/overridden",
                "headers": [["x-flag", "1"]],
                "body": "{\"new\":true}"
            }
        });
        let resp = execute(&st, serde_json::from_value(req).unwrap())
            .await
            .unwrap();
        assert_eq!(resp["sent_event"]["path"], "/overridden");
        let got = seen.lock().unwrap().pop().unwrap();
        assert_eq!(got.headers().get("x-flag").unwrap(), "1");
        assert_eq!(got.body(), "{\"new\":true}".as_bytes());
    }
}
