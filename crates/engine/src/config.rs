//! Engine configuration: flags (CLI) override env vars override defaults.
//! The CLI owns parsing; the engine owns semantics.

use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Control plane: `/api/*`, `/ws`, SSE. Default `127.0.0.1:8000`.
    pub control_addr: SocketAddr,
    /// Capture plane: webhook ingestion. Default `127.0.0.1:8001`.
    pub capture_addr: SocketAddr,
    /// When set, every control-plane request needs `Authorization: Bearer <token>`.
    pub token: Option<String>,
    /// Hosted mode: accept `{slug}.{capture_zone}` hosts on the capture plane.
    pub capture_zone: Option<String>,
    /// Per-request capture body limit; larger bodies are truncated (envelope `truncated`).
    pub max_body_bytes: usize,
    /// Optional engine config file (signing profiles live here, never in the API).
    pub config_path: Option<PathBuf>,
    /// Optional SQLite database: events + channels survive restarts (§ M2).
    pub db_path: Option<PathBuf>,
    /// Enable the billing layer (plans, quota enforcement, checkout, webhooks).
    /// Secrets come from `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET`.
    /// Hard channel cap without billing (defaults to none).
    pub max_channels: Option<usize>,
    pub billing_enabled: bool,
    /// Stripe Price ID for the Pro plan (needed for checkout sessions).
    pub stripe_price_pro: Option<String>,
    /// Webhook signing secret; falls back to the `STRIPE_WEBHOOK_SECRET` env var.
    pub stripe_webhook_secret: Option<String>,
    /// Permit inline `secret` in replay requests (CI convenience; never echoed back).
    pub allow_inline_secret: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            control_addr: SocketAddr::from(([127, 0, 0, 1], 8000)),
            capture_addr: SocketAddr::from(([127, 0, 0, 1], 8001)),
            token: None,
            capture_zone: None,
            max_body_bytes: 1024 * 1024,
            config_path: None,
            db_path: None,
            max_channels: None,
            billing_enabled: false,
            stripe_price_pro: None,
            stripe_webhook_secret: None,
            allow_inline_secret: false,
        }
    }
}
