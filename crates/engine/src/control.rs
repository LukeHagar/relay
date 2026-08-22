//! Control plane (§2–§6): channel management, history, `/ws`, SSE, auth middleware.

use crate::hub::{ServerFrame, SUBSCRIPTION_BUFFER};
use crate::store::{ChannelConfig, FromSpec};
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures::stream::{FusedStream, SelectAll};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .fallback(static_handler)
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route(
            "/api/openapi.yaml",
            get(|| async {
                (
                    [(http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
                    include_str!("../../../docs/openapi.yaml"),
                )
            }),
        )
        .route("/api/channels", get(list_channels))
        .route(
            "/api/channels/{name}",
            get(get_channel).put(upsert_channel).delete(delete_channel),
        )
        .route("/api/channels/{name}/events", get(events))
        .route("/api/channels/{name}/sse", get(sse))
        .route("/api/replay", axum::routing::post(crate::replay::handler))
        .route("/api/billing/plans", get(billing_plans))
        .route("/api/billing/subscription", get(billing_subscription))
        .route(
            "/api/billing/checkout",
            axum::routing::post(billing_checkout),
        )
        .route("/api/billing/webhook", axum::routing::post(billing_webhook))
        .route("/api/templates", get(list_templates))
        .route("/api/signing-profiles", get(list_signing_profiles))
        .route("/ws", get(ws_upgrade))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({"ok": true, "version": env!("CARGO_PKG_VERSION")}))
}

/// Liveness vs readiness: `readyz` additionally proves the durable layer answers.
async fn readyz(State(state): State<crate::AppState>) -> Response {
    let db_ok = match &state.db {
        None => true,
        Some(db) => {
            let probe = || {
                db.conn
                    .lock()
                    .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                    .map(|v| v == 1)
            };
            probe().unwrap_or(false)
        }
    };
    if db_ok {
        Json(json!({"ready": true})).into_response()
    } else {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ready": false, "reason": "database unavailable"})),
        )
            .into_response()
    }
}

/// §2: when `--token` is set, every control-plane request needs
/// `Authorization: Bearer <t>`; `/ws` and SSE also accept `?token=` (browsers).
async fn auth(State(state): State<crate::AppState>, req: Request<Body>, next: Next) -> Response {
    // Stripe webhooks authenticate by signature, not by our bearer token.
    if req.uri().path() == "/api/billing/webhook" {
        return next.run(req).await;
    }
    let Some(expected) = &state.config.token else {
        return next.run(req).await;
    };
    let header_ok = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {expected}"));
    let query_ok = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find_map(|kv| kv.strip_prefix("token="))
                .map(|t| t == expected)
        })
        .unwrap_or(false);
    if header_ok || query_ok {
        next.run(req).await
    } else {
        (axum::http::StatusCode::UNAUTHORIZED, Json(json!({"error": {"code": "unauthorized", "message": "control-plane token missing or wrong"}}))).into_response()
    }
}

fn channel_descriptor(
    state: &crate::AppState,
    ch: &Arc<crate::store::Channel>,
) -> serde_json::Value {
    let cfg = ch.config();
    json!({
        "name": ch.name,
        "created_at": ch.created_at,
        "response_mode": cfg.response_mode(),
        "buffer": cfg.buffer(),
        "auth": cfg.auth,
        "rate_limit": cfg.rate_limit(),
        "forward": cfg.forward,
        "capture_url_local": format!("http://{}{}/c/{}", state.config.capture_addr, "", ch.name),
        "capture_url_hosted": state.config.capture_zone.as_ref().map(|z| format!("https://{}.{z}", ch.name)),
    })
}

async fn list_channels(State(state): State<crate::AppState>) -> Response {
    let channels: Vec<serde_json::Value> = state
        .store
        .list()
        .iter()
        .map(|c| channel_descriptor(&state, c))
        .collect();
    Json(json!({ "channels": channels })).into_response()
}

async fn get_channel(State(state): State<crate::AppState>, Path(name): Path<String>) -> Response {
    match state.store.get(&name) {
        Some(ch) => Json(channel_descriptor(&state, &ch)).into_response(),
        None => not_found_channel(&name),
    }
}

async fn upsert_channel(
    State(state): State<crate::AppState>,
    Path(name): Path<String>,
    body: String,
) -> Response {
    let config: ChannelConfig = if body.trim().is_empty() {
        ChannelConfig::default()
    } else {
        match serde_json::from_str(&body) {
            Ok(c) => c,
            Err(e) => {
                return bad_request(format!("invalid channel config: {e}"));
            }
        }
    };
    if let Some(cap) = state.config.max_channels {
        let existing = state.store.list().len();
        let updating = state.store.get(&name).is_some();
        if !updating && existing >= cap {
            return (
                axum::http::StatusCode::PAYMENT_REQUIRED,
                Json(json!({"error": {"code": "quota_exceeded", "message": format!(
                    "--max-channels is {cap} and {existing} channels exist; raise the flag or delete one"
                )}})),
            )
                .into_response();
        }
    }
    if state.config.billing_enabled {
        let plan = state.billing.subscription.lock().plan().clone();
        let existing = state.store.list().len();
        let updating = state.store.get(&name).is_some();
        if !updating && !plan.allows_channels(existing) {
            return (
                axum::http::StatusCode::PAYMENT_REQUIRED,
                Json(json!({"error": {"code": "quota_exceeded", "message": format!(
                    "plan '{}' allows {} channel(s); upgrade to add more", plan.id,
                    if plan.max_channels == usize::MAX { "unlimited".to_string() } else { plan.max_channels.to_string() }
                )}})),
            )
                .into_response();
        }
        if !plan.allows_forward_rules(config.forward.len()) {
            return (
                axum::http::StatusCode::PAYMENT_REQUIRED,
                Json(
                    json!({"error": {"code": "quota_exceeded", "message": format!(
                        "plan '{}' does not include forwarding rules; upgrade to use them", plan.id
                    )}}),
                ),
            )
                .into_response();
        }
    }
    match state.store.upsert(&name, config) {
        Ok(res) => {
            if let Some(db) = &state.db {
                let cfg = res.channel.config();
                if let Err(e) = db.insert_channel(&name, &cfg, &res.channel.created_at) {
                    tracing::error!(error = %e, "failed to persist channel");
                }
            }
            let status = if res.created {
                axum::http::StatusCode::CREATED
            } else {
                axum::http::StatusCode::OK
            };
            (status, Json(channel_descriptor(&state, &res.channel))).into_response()
        }
        Err(e) => bad_request(e.to_string()),
    }
}

async fn delete_channel(
    State(state): State<crate::AppState>,
    Path(name): Path<String>,
) -> Response {
    match state.store.remove(&name) {
        Some(_) => {
            state.hub.close_channel(&name);
            if let Some(db) = &state.db {
                if let Err(e) = db.delete_channel(&name) {
                    tracing::error!(error = %e, "failed to delete persisted channel");
                }
            }
            axum::http::StatusCode::NO_CONTENT.into_response()
        }
        None => not_found_channel(&name),
    }
}

#[derive(serde::Deserialize)]
struct HistoryQuery {
    cursor: Option<u64>,
    limit: Option<usize>,
}

async fn events(
    State(state): State<crate::AppState>,
    Path(name): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Response {
    let Some(ch) = state.store.get(&name) else {
        return not_found_channel(&name);
    };
    let limit = q.limit.unwrap_or(1000).min(10_000);
    // The durable layer is authoritative when present; otherwise the ring buffer.
    let events: Vec<serde_json::Value> = match &state.db {
        Some(db) => match db.events(&name, q.cursor, limit) {
            Ok(evts) => evts
                .iter()
                .map(|e| serde_json::to_value(e).unwrap())
                .collect(),
            Err(e) => {
                tracing::error!(error = %e, "db history read failed");
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": {"code": "history_unavailable",
                        "message": format!("durable history read failed: {e}")}})),
                )
                    .into_response();
            }
        },
        None => ch
            .history(q.cursor, limit)
            .iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .collect(),
    };
    let last_seq = events.last().and_then(|v| v["seq"].as_u64());
    Json(json!({ "events": events, "last_seq": last_seq })).into_response()
}

// ---------------------------------------------------------------- WebSocket

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientFrame {
    Subscribe {
        id: String,
        channel: String,
        #[serde(default = "default_from")]
        from: serde_json::Value,
    },
    Unsubscribe {
        id: String,
        channel: String,
    },
    Ping {
        id: String,
    },
}

fn default_from() -> serde_json::Value {
    json!("newest")
}

async fn ws_upgrade(State(state): State<crate::AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: crate::AppState, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let mut streams: SelectAll<ReceiverStream<ServerFrame>> = SelectAll::new();

    loop {
        tokio::select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(t) => {
                        if !handle_client_frame(&state, &mut sink, &mut streams, &t).await {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    Message::Ping(p) => {
                        if sink.send(Message::Pong(p)).await.is_err() {
                            break;
                        }
                    }
                    Message::Binary(_) | Message::Pong(_) => {}
                }
            }
            frame = streams.next(), if !streams.is_terminated() => {
                let Some(frame) = frame else { continue };
                let close_after =
                    matches!(frame, ServerFrame::Error { ref error, .. } if error.code == "slow_consumer");
                let text = serde_json::to_string(&frame).unwrap();
                if sink.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
                if close_after {
                    break; // §5.1 item 4: error frame, then close
                }
            }
        }
    }
    let _ = sink.close().await;
}

/// Returns false when the connection should terminate.
async fn handle_client_frame(
    state: &crate::AppState,
    sink: &mut (impl futures::Sink<Message, Error = axum::Error> + Unpin),
    streams: &mut SelectAll<ReceiverStream<ServerFrame>>,
    text: &str,
) -> bool {
    let frame: ClientFrame = match serde_json::from_str(text) {
        Ok(f) => f,
        Err(e) => {
            return send_server(sink, ServerFrame::error(None, "bad_request", e.to_string())).await;
        }
    };
    match frame {
        ClientFrame::Ping { id } => send_server(sink, ServerFrame::pong(id)).await,
        ClientFrame::Unsubscribe { id, channel } => {
            state.hub.unsubscribe(&id, &channel);
            true // hub dropped its sender; SelectAll prunes the finished stream
        }
        ClientFrame::Subscribe { id, channel, from } => {
            let Some(ch) = state.store.get(&channel) else {
                return send_server(
                    sink,
                    ServerFrame::error(
                        Some(id),
                        "no_such_channel",
                        format!("channel '{channel}' does not exist"),
                    ),
                )
                .await;
            };
            let Some(from) = FromSpec::parse(from) else {
                return send_server(
                    sink,
                    ServerFrame::error(
                        Some(id),
                        "bad_request",
                        "from must be newest, oldest, or a sequence number",
                    ),
                )
                .await;
            };
            let (tx, rx) = mpsc::channel::<ServerFrame>(SUBSCRIPTION_BUFFER);
            let last_seq = ch.last_seq();
            let backlog = state.hub.subscribe(&ch, &id, &from, tx.clone());
            // §5.1 item 1: ok + backlog first; live frames buffer in rx meanwhile.
            if !send_server(sink, ServerFrame::ok(id.clone(), last_seq)).await {
                return false;
            }
            for e in backlog {
                if !send_server(sink, ServerFrame::event(id.clone(), e)).await {
                    return false;
                }
            }
            streams.push(ReceiverStream::new(rx));
            true
        }
    }
}

async fn send_server(
    sink: &mut (impl futures::Sink<Message, Error = axum::Error> + Unpin),
    frame: ServerFrame,
) -> bool {
    let text = serde_json::to_string(&frame).unwrap();
    sink.send(Message::Text(text.into())).await.is_ok()
}

// ---------------------------------------------------------------- SSE

#[derive(serde::Deserialize)]
struct SseQuery {
    from: Option<String>,
    /// Cap the replayed backlog (live events are unaffected).
    limit: Option<usize>,
}

async fn sse(
    State(state): State<crate::AppState>,
    Path(name): Path<String>,
    Query(q): Query<SseQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    let Some(ch) = state.store.get(&name) else {
        return not_found_channel(&name);
    };
    // `Last-Event-ID` is honored as the cursor when `from` is absent (§5.2).
    let from_value: serde_json::Value = match (&q.from, headers.get("last-event-id")) {
        (Some(f), _) => json!(f),
        (None, Some(lei)) => match lei.to_str().ok().and_then(|s| s.parse::<u64>().ok()) {
            Some(seq) => json!(seq),
            None => json!("oldest"),
        },
        (None, None) => json!("oldest"),
    };
    let Some(from) = FromSpec::parse(from_value) else {
        return bad_request("from must be \"newest\", \"oldest\", or a sequence number");
    };

    let (tx, mut rx) = mpsc::channel::<ServerFrame>(SUBSCRIPTION_BUFFER);
    let backlog = state
        .hub
        .subscribe(&ch, &format!("sse-{}", ulid::Ulid::new()), &from, tx);
    let _ = q.limit; // backlog is buffer-bounded; limit applies to /events only

    let stream = async_stream::stream! {
        for e in backlog {
            yield Ok::<_, std::convert::Infallible>(sse_event(&e));
        }
        while let Some(frame) = rx.recv().await {
            match frame {
                ServerFrame::Event { event, .. } => yield Ok(sse_event(&event)),
                ServerFrame::Error { error, .. } if error.code == "slow_consumer" => break,
                ServerFrame::ChannelClosed { .. } => break,
                _ => {}
            }
        }
    };

    axum::response::sse::Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text(""),
        )
        .into_response()
}

fn sse_event(env: &crate::envelope::Envelope) -> axum::response::sse::Event {
    axum::response::sse::Event::default()
        .id(env.seq.to_string())
        .event("relay.event")
        .data(serde_json::to_string(env).unwrap())
}

fn not_found_channel(name: &str) -> Response {
    (
        axum::http::StatusCode::NOT_FOUND,
        Json(json!({"error": {"code": "no_such_channel", "message": format!("channel '{name}' does not exist")}})),
    )
        .into_response()
}

fn bad_request(message: impl Into<String>) -> Response {
    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(json!({"error": {"code": "bad_request", "message": message.into()}})),
    )
        .into_response()
}

async fn list_templates(State(state): State<crate::AppState>) -> Response {
    let items: Vec<serde_json::Value> = state
        .templates
        .list()
        .iter()
        .map(|t| {
            json!({
                "ref": t.r#ref(),
                "provider": t.provider,
                "name": t.name,
                "description": t.description,
                "signing_hint": t.signing_hint,
                "defaults": t.defaults.vars,
            })
        })
        .collect();
    Json(json!({ "templates": items })).into_response()
}

async fn list_signing_profiles(State(state): State<crate::AppState>) -> Response {
    let names: Vec<&String> = state.profiles.0.keys().collect();
    Json(json!({ "profiles": names })).into_response()
}

// ---------------------------------------------------------------- billing

async fn billing_plans(State(state): State<crate::AppState>) -> Response {
    let current = state.billing.subscription.lock().clone();
    Json(json!({
        "billing_enabled": state.config.billing_enabled,
        "plans": crate::billing::PLANS,
        "subscription": current,
    }))
    .into_response()
}

async fn billing_subscription(State(state): State<crate::AppState>) -> Response {
    Json(state.billing.subscription.lock().clone()).into_response()
}

#[derive(serde::Deserialize)]
struct CheckoutBody {
    success_url: String,
    cancel_url: String,
}

async fn billing_checkout(State(state): State<crate::AppState>, body: String) -> Response {
    if !state.config.billing_enabled {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"error": {"code": "billing_disabled", "message": "start the engine with billing enabled"}})),
        ).into_response();
    }
    let Some(price) = state.config.stripe_price_pro.clone() else {
        return (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": {"code": "billing_not_configured", "message": "RELAY_STRIPE_PRICE_PRO is not set"}})),
        ).into_response();
    };
    let parsed: CheckoutBody = match serde_json::from_str(&body) {
        Ok(b) => b,
        Err(e) => return bad_request(format!("invalid checkout request: {e}")),
    };
    match crate::billing::create_checkout_session(
        &state.http,
        &price,
        &parsed.success_url,
        &parsed.cancel_url,
    )
    .await
    {
        Ok(url) => Json(json!({ "url": url })).into_response(),
        Err(e) => {
            let msg = e.to_string();
            let (status, code) = if msg.contains("STRIPE_SECRET_KEY") {
                (
                    axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                    "billing_not_configured",
                )
            } else {
                (axum::http::StatusCode::BAD_GATEWAY, "stripe_error")
            };
            (
                status,
                Json(json!({"error": {"code": code, "message": msg}})),
            )
                .into_response()
        }
    }
}

/// Stripe delivers here. Authenticated by signature over the raw body (§6.1 scheme),
/// never by our bearer token.
async fn billing_webhook(
    State(state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    raw: axum::body::Bytes,
) -> Response {
    let secret = state
        .config
        .stripe_webhook_secret
        .clone()
        .or_else(|| std::env::var("STRIPE_WEBHOOK_SECRET").ok())
        .unwrap_or_default();
    if secret.is_empty() {
        return (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": {"code": "billing_not_configured", "message": "STRIPE_WEBHOOK_SECRET is not set"}})),
        ).into_response();
    }
    let header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let now = crate::envelope::now_utc().unix_timestamp();
    if let Err(outcome) = crate::billing::verify_stripe_header(header, &raw, &secret, now) {
        let code = match outcome {
            crate::billing::WebhookOutcome::StaleTimestamp => "stale_timestamp",
            _ => "invalid_signature",
        };
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"error": {"code": code, "message": "webhook signature check failed"}})),
        )
            .into_response();
    }
    let event: serde_json::Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(e) => return bad_request(format!("invalid webhook payload: {e}")),
    };
    match crate::billing::apply_stripe_event(state.db.as_ref(), &state.billing.subscription, &event)
    {
        crate::billing::WebhookOutcome::Received | crate::billing::WebhookOutcome::Unhandled(_) => {
            Json(json!({ "received": true })).into_response()
        }
        _ => unreachable!("verify already rejected these"),
    }
}

// ---------------------------------------------------------------- UI assets

#[derive(rust_embed::RustEmbed)]
#[folder = "../../app/build"]
struct UiAssets;

/// Serve the SvelteKit UI: `/` → index.html, exact asset paths, and an index
/// fallback for deep links (SPA mode). Unknown `/api/*` paths still 404 as JSON.
async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") {
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({"error": {"code": "no_such_route", "message": "unknown API route"}})),
        )
            .into_response();
    }
    let file = if path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    let served =
        UiAssets::get(file).or_else(|| UiAssets::get("index.html").filter(|_| !file.contains('.')));
    match served {
        Some(content) => {
            let mime = mime_guess::from_path(file).first_or_octet_stream();
            (
                [(http::header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data,
            )
                .into_response()
        }
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({"error": {"code": "no_such_route", "message": "not found"}})),
        )
            .into_response(),
    }
}
