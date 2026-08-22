//! Relay engine: local-first webhook capture, inspection, and signed replay.
//!
//! Two planes, two listeners (see `docs/api.md`):
//! - control plane: `/api/*` management API, `/ws` subscriptions, SSE
//! - capture plane: webhook ingestion only

pub mod billing;
pub mod capture;
pub mod config;
pub mod control;
pub mod envelope;
pub mod forwarding;
pub mod hub;
pub mod replay;
pub mod signing;
pub mod storage;
pub mod store;
pub mod templates;

/// Bundled provider starter sets (§7), embedded at build time.
pub const BUNDLED_TEMPLATES: &[&str] = &[
    include_str!("../../../templates/github/push.json"),
    include_str!("../../../templates/github/pull_request.opened.json"),
    include_str!("../../../templates/github/issues.opened.json"),
    include_str!("../../../templates/stripe/invoice.paid.json"),
    include_str!("../../../templates/stripe/checkout.session.completed.json"),
    include_str!("../../../templates/stripe/customer.subscription.deleted.json"),
    include_str!("../../../templates/slack/message.channels.json"),
    include_str!("../../../templates/slack/url_verification.json"),
    include_str!("../../../templates/shopify/order_created.json"),
    include_str!("../../../templates/twilio/message.received.json"),
];

use crate::config::Config;
use axum::response::IntoResponse;
use axum::serve::ListenerExt;
use axum::{Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub store: Arc<store::Store>,
    pub hub: Arc<hub::Hub>,
    /// Signing profiles from engine config — secrets resolve out-of-band (§6).
    pub profiles: Arc<signing::Profiles>,
    /// Bundled + user template fixtures (§7).
    pub templates: Arc<templates::Registry>,
    /// Shared HTTP client for replays.
    pub http: reqwest::Client,
    /// Optional durable layer (`--db`).
    pub db: Option<Arc<storage::Db>>,
    /// Subscription state for this instance (billing layer).
    pub billing: Arc<billing::Billing>,
    /// Durable-layer write failures since boot (surfaced via `x-relay-persisted`).
    pub persisted_failures: Arc<std::sync::atomic::AtomicU64>,
}

impl AppState {
    /// Fallible constructor: an explicit `--config` file must parse or we refuse
    /// to start (silent misconfiguration would break replays later).
    pub fn build(config: Config) -> anyhow::Result<Self> {
        let profiles = match &config.config_path {
            Some(path) => signing::Profiles::load_file(path)?,
            None => signing::Profiles::default(),
        };
        let templates_dir = std::path::Path::new("templates");
        let templates = if templates_dir.is_dir() {
            templates::Registry::load(BUNDLED_TEMPLATES, Some(templates_dir))?
        } else {
            templates::Registry::load(BUNDLED_TEMPLATES, None)?
        };
        let store = Arc::new(store::Store::default());
        let db = match &config.db_path {
            Some(path) => {
                let db = storage::Db::open(path)?;
                for loaded in db.load_channels()? {
                    store.restore(
                        &loaded.name,
                        loaded.config,
                        loaded.created_at,
                        loaded.last_allocated_seq,
                    )?;
                }
                Some(db)
            }
            None => None,
        };
        let billing = Arc::new(billing::Billing::default());
        let persisted_failures = Arc::new(std::sync::atomic::AtomicU64::new(0));
        if let Some(db) = &db {
            // Restore the persisted subscription so entitlements survive restarts.
            if let Some(sub) = db.load_subscription()? {
                *billing.subscription.lock() = sub;
            }
        }
        // Tuned pool: replay bursts and forwarding fan-out reuse warm connections
        // instead of paying TCP+TLS setup per delivery.
        let http = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(64)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_nodelay(true)
            .connect_timeout(std::time::Duration::from_secs(10))
            .user_agent(concat!("relay-engine/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            config,
            store,
            hub: Arc::new(hub::Hub::default()),
            profiles: Arc::new(profiles),
            templates: Arc::new(templates),
            http,
            db,
            billing,
            persisted_failures,
        })
    }

    /// Infallible variant for tests/embedders using defaults only.
    pub fn new(config: Config) -> Self {
        AppState::build(config).expect("bundled engine state builds")
    }
}

pub fn control_router(state: AppState) -> Router {
    harden(control::router(state), std::time::Duration::from_secs(15))
}

pub fn capture_router(state: AppState) -> Router {
    harden(capture::router(state), std::time::Duration::from_secs(35))
}

/// Panic isolation + per-request timeouts on every plane: a handler bug costs one
/// 500, never a wedged worker; a stuck client costs one 504, never a slowloris.
/// Plain middleware keeps the router's `Infallible` error type intact.
async fn plane_guard(
    duration: std::time::Duration,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // Replays carry their own, longer budget (timeout_ms up to 60s); the plane-wide
    // 15s guard must not cut them mid-flight and leave delivery state ambiguous.
    let duration = if req.uri().path() == "/api/replay" {
        std::time::Duration::from_secs(75)
    } else {
        duration
    };
    use futures::FutureExt;
    let fut = std::panic::AssertUnwindSafe(next.run(req));
    match tokio::time::timeout(duration, fut.catch_unwind()).await {
        Err(_) => (
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({"error": {"code": "request_failed", "message": "request timed out"}})),
        )
            .into_response(),
        Ok(Err(_panic)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": {"code": "internal_error",
                "message": "handler panicked; the engine kept running"}})),
        )
            .into_response(),
        Ok(Ok(resp)) => resp,
    }
}

fn harden(route: Router, duration: std::time::Duration) -> Router {
    route.layer(axum::middleware::from_fn(
        move |req: axum::extract::Request, next: axum::middleware::Next| {
            plane_guard(duration, req, next)
        },
    ))
}

async fn shutdown_signal(token: tokio_util::sync::CancellationToken) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = token.cancelled() => {},
    }
    // A signal (or external cancel) must propagate: the per-listener graceful
    // shutdown futures wait on child tokens derived from this one.
    token.cancel();
}

/// Bind both planes and run until shutdown (Ctrl+C / SIGTERM).
pub async fn serve(cfg: Config) -> anyhow::Result<()> {
    let state = AppState::build(cfg.clone())?;
    let control = control_router(state.clone());
    let capture = capture_router(state);

    let control_listener = tokio::net::TcpListener::bind(cfg.control_addr).await?;
    let capture_listener = tokio::net::TcpListener::bind(cfg.capture_addr).await?;
    tracing::info!(
        control = %cfg.control_addr,
        capture = %cfg.capture_addr,
        "relay engine listening"
    );

    let root = tokio_util::sync::CancellationToken::new();
    let installer = tokio::spawn({
        let root = root.clone();
        async move { shutdown_signal(root).await }
    });

    let control_token = root.child_token();
    let capture_token = root.child_token();

    // Linux accepted sockets inherit TCP_NODELAY from the listener — without this,
    // Nagle + delayed-ACK interactions explain the multi-ms tail under load.
    let control_listener = control_listener.tap_io(|io| {
        let _ = io.set_nodelay(true);
    });
    let capture_listener = capture_listener.tap_io(|io| {
        let _ = io.set_nodelay(true);
    });

    let control_task = tokio::spawn(async move {
        axum::serve(
            control_listener,
            control.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { control_token.cancelled().await })
        .await
    });
    let capture_task = tokio::spawn(async move {
        axum::serve(
            capture_listener,
            capture.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { capture_token.cancelled().await })
        .await
    });

    let (control_res, capture_res) = tokio::join!(control_task, capture_task);
    installer.abort();
    control_res??;
    capture_res??;
    Ok(())
}
