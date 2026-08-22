//! Stripe billing (M4 SaaS layer): plan catalog, entitlement gating, checkout
//! sessions, and webhook-driven subscription state.
//!
//! Design notes:
//! - Secrets never live in config files: `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET`
//!   come from the environment.
//! - Webhook signatures are verified with the *same* Stripe canonical-string scheme as
//!   `signing::Scheme::Stripe` (`t=<ts>,v1=hex(HMAC(secret, "<t>.<body>"))`), plus a
//!   replay-tolerance window — the engine eats its own dog food.
//! - Everything except `POST /v1/checkout/sessions` is offline-testable; the HTTP call
//!   is isolated behind [`create_checkout_session`] and requires a configured key.

use crate::storage::Db;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

// ---------------------------------------------------------------- plans

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub id: &'static str,
    pub name: &'static str,
    /// `usize::MAX` = unlimited.
    pub max_channels: usize,
    /// Forwarding rules per channel.
    pub max_forward_rules: usize,
    pub price_monthly_usd: f64,
}

pub const FREE_PLAN: &str = "free";
pub const PRO_PLAN: &str = "pro";

pub const PLANS: &[Plan] = &[
    Plan {
        id: FREE_PLAN,
        name: "Free",
        max_channels: 5,
        max_forward_rules: 0,
        price_monthly_usd: 0.0,
    },
    Plan {
        id: PRO_PLAN,
        name: "Pro",
        max_channels: usize::MAX,
        max_forward_rules: usize::MAX,
        price_monthly_usd: 12.0,
    },
];

pub fn plan_by_id(id: &str) -> Option<&'static Plan> {
    PLANS.iter().find(|p| p.id == id)
}

impl Plan {
    pub fn allows_channels(&self, current: usize) -> bool {
        current < self.max_channels
    }
    pub fn allows_forward_rules(&self, requested: usize) -> bool {
        requested <= self.max_forward_rules
    }
}

// ---------------------------------------------------------------- state

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubStatus {
    #[default]
    Active,
    Trialing,
    PastDue,
    Canceled,
}

/// The engine instance's own subscription. Single-tenant by design: an operator
/// upgrades *this instance*; tenant accounts are a deployment-layer concern.
#[derive(Debug, Clone, Serialize)]
pub struct Subscription {
    pub plan_id: String,
    pub status: SubStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_subscription_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end_unix: Option<i64>,
}

impl Default for Subscription {
    /// Fresh instance: on the Free plan, active (Free limits are the entitlements).
    fn default() -> Self {
        Self {
            plan_id: FREE_PLAN.into(),
            status: SubStatus::Active,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            current_period_end_unix: None,
        }
    }
}

impl Subscription {
    pub fn free() -> Self {
        Self::default()
    }
    pub fn entitled(&self) -> bool {
        self.status == SubStatus::Active || self.status == SubStatus::Trialing
    }
    pub fn plan(&self) -> &'static Plan {
        let id = if self.entitled() {
            self.plan_id.as_str()
        } else {
            FREE_PLAN
        };
        plan_by_id(id).unwrap_or(&PLANS[0])
    }
}

/// Shared handle in `AppState`.
#[derive(Default)]
pub struct Billing {
    pub subscription: Mutex<Subscription>,
}

// ---------------------------------------------------------------- persistence

const SUB_KEY: &str = "instance";

impl Db {
    pub fn load_subscription(&self) -> anyhow::Result<Option<Subscription>> {
        let conn = self.conn.lock();
        let exists = conn
            .query_row(
                "SELECT 1 FROM subscriptions WHERE id = ?1",
                [SUB_KEY],
                |_| Ok(()),
            )
            .is_ok();
        if !exists {
            return Ok(None);
        }
        let mut stmt = conn.prepare(
            "SELECT plan_id, status, stripe_customer_id, stripe_subscription_id, current_period_end_unix
             FROM subscriptions WHERE id = ?1",
        )?;
        let row = stmt.query_row([SUB_KEY], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?;
        Ok(Some(Subscription {
            plan_id: row.0,
            status: serde_json::from_str(&format!("\"{}\"", row.1))?,
            stripe_customer_id: row.2,
            stripe_subscription_id: row.3,
            current_period_end_unix: row.4,
        }))
    }

    pub fn save_subscription(&self, sub: &Subscription) -> anyhow::Result<()> {
        self.conn.lock().execute(
            "INSERT INTO subscriptions (id, plan_id, status, stripe_customer_id, stripe_subscription_id, current_period_end_unix)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET plan_id=excluded.plan_id, status=excluded.status,
               stripe_customer_id=excluded.stripe_customer_id, stripe_subscription_id=excluded.stripe_subscription_id,
               current_period_end_unix=excluded.current_period_end_unix",
            rusqlite::params![
                SUB_KEY,
                sub.plan_id,
                format!("{:?}", sub.status).to_lowercase(),
                sub.stripe_customer_id,
                sub.stripe_subscription_id,
                sub.current_period_end_unix,
            ],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------- webhooks

pub const WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Outcome of verifying + applying a Stripe webhook delivery.
pub enum WebhookOutcome {
    Received,
    InvalidSignature,
    StaleTimestamp,
    Unhandled(String),
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify a `Stripe-Signature` header against the raw body (§6.1 construction +
/// replay tolerance). Independent of network.
pub fn verify_stripe_header(
    header: &str,
    body: &[u8],
    secret: &str,
    now_unix: i64,
) -> Result<(), WebhookOutcome> {
    let (t, v1) = parse_header(header).ok_or(WebhookOutcome::InvalidSignature)?;
    if (now_unix - t).abs() > WEBHOOK_TOLERANCE_SECS {
        return Err(WebhookOutcome::StaleTimestamp);
    }
    let expected = crate::signing::sign_for(crate::signing::Scheme::Stripe, secret, body, t, None);
    let (_, expected_sig) = expected[0].clone();
    let expected_hex = expected_sig.rsplit('=').next().unwrap_or("");
    if constant_time_eq(v1.as_bytes(), expected_hex.as_bytes()) {
        Ok(())
    } else {
        Err(WebhookOutcome::InvalidSignature)
    }
}

fn parse_header(header: &str) -> Option<(i64, String)> {
    let mut t = None;
    let mut v1 = None;
    for part in header.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("t=") {
            t = v.parse().ok();
        } else if let Some(v) = part.strip_prefix("v1=") {
            v1 = Some(v.to_string());
        }
    }
    Some((t?, v1?))
}

/// Apply a verified Stripe event to subscription state (and persist). Factored out of
/// the HTTP handler so tests drive transitions without the network.
pub fn apply_stripe_event(
    db: Option<&Arc<Db>>,
    sub: &Mutex<Subscription>,
    event: &serde_json::Value,
) -> WebhookOutcome {
    let event_type = event["type"].as_str().unwrap_or_default().to_string();
    let obj = &event["data"]["object"];
    let mut guard = sub.lock();
    match event_type.as_str() {
        "checkout.session.completed" => {
            // We stamp the purchased plan into checkout metadata at creation time.
            let plan = obj["metadata"]["plan"].as_str().unwrap_or(PRO_PLAN);
            guard.plan_id = plan.to_string();
            guard.status = SubStatus::Active;
            guard.stripe_customer_id = obj["customer"].as_str().map(str::to_string);
            guard.stripe_subscription_id = obj["subscription"].as_str().map(str::to_string);
        }
        "customer.subscription.updated" => {
            guard.status = map_sub_status(obj["status"].as_str().unwrap_or("canceled"));
            guard.current_period_end_unix = obj["current_period_end"].as_i64();
        }
        "customer.subscription.deleted" => {
            guard.plan_id = FREE_PLAN.into();
            guard.status = SubStatus::Canceled;
            guard.stripe_subscription_id = None;
        }
        other => return WebhookOutcome::Unhandled(other.to_string()),
    }
    if let Some(db) = db {
        if let Err(e) = db.save_subscription(&guard.clone()) {
            tracing::error!(error = %e, "failed to persist subscription");
        }
    }
    WebhookOutcome::Received
}

fn map_sub_status(s: &str) -> SubStatus {
    match s {
        "active" => SubStatus::Active,
        "trialing" => SubStatus::Trialing,
        "past_due" | "unpaid" => SubStatus::PastDue,
        _ => SubStatus::Canceled,
    }
}

// ---------------------------------------------------------------- checkout client

/// Build the form-encoded body for a Stripe Checkout Session (unit-tested; the HTTP
/// call itself needs a real key and is exercised only against Stripe).
pub fn build_checkout_form(
    price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> Vec<(String, String)> {
    vec![
        ("mode".into(), "subscription".into()),
        ("line_items[0][price]".into(), price_id.into()),
        ("line_items[0][quantity]".into(), "1".into()),
        ("metadata[plan]".into(), PRO_PLAN.into()),
        ("success_url".into(), success_url.into()),
        ("cancel_url".into(), cancel_url.into()),
        ("allow_promotion_codes".into(), "true".into()),
    ]
}

/// Create a Checkout Session via the Stripe REST API. Requires `STRIPE_SECRET_KEY`.
pub async fn create_checkout_session(
    http: &reqwest::Client,
    price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> anyhow::Result<String> {
    let key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|_| anyhow::anyhow!("STRIPE_SECRET_KEY is not set"))?;
    let form = build_checkout_form(price_id, success_url, cancel_url);
    let resp = http
        .post("https://api.stripe.com/v1/checkout/sessions")
        .bearer_auth(key)
        .form(&form)
        .send()
        .await?;
    let value: serde_json::Value = resp.json().await?;
    value["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("stripe response missing url: {value}"))
}

/// Convenience JSON error payload builder shared with the control plane.
pub fn billing_error(code: &str, message: impl Into<String>) -> serde_json::Value {
    json!({ "error": { "code": code, "message": message.into() } })
}
