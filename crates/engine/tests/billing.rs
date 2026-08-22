//! Billing tests (M4): Stripe webhook signature verification, subscription state
//! transitions, and plan-based entitlement gating — all offline.

use parking_lot::Mutex;
use relay_engine::billing::{
    apply_stripe_event, build_checkout_form, plan_by_id, verify_stripe_header, Billing,
    WebhookOutcome, FREE_PLAN, PLANS, PRO_PLAN,
};
use relay_engine::config::Config;
use relay_engine::hub::Hub;
use relay_engine::signing::sign_for;
use relay_engine::storage::Db;
use relay_engine::store::Store;
use relay_engine::AppState;
use std::sync::Arc;
use tower::ServiceExt;

const SECRET: &str = "whsec_test_secret";

fn state_with_billing(enabled: bool, db: Option<Arc<Db>>) -> AppState {
    state_with_billing_and_secret(enabled, db, Some(SECRET.into()))
}

fn state_with_billing_and_secret(
    enabled: bool,
    db: Option<Arc<Db>>,
    secret: Option<String>,
) -> AppState {
    AppState {
        config: Config {
            billing_enabled: enabled,
            stripe_webhook_secret: secret,
            ..Default::default()
        },
        store: Arc::new(Store::default()),
        hub: Arc::new(Hub::default()),
        profiles: Arc::new(relay_engine::signing::Profiles::default()),
        templates: Arc::new(
            relay_engine::templates::Registry::load(relay_engine::BUNDLED_TEMPLATES, None).unwrap(),
        ),
        http: reqwest::Client::new(),
        db,
        persisted_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        billing: Arc::new(Billing::default()),
    }
}

fn signed_header(body: &[u8], t: i64) -> String {
    sign_for(relay_engine::signing::Scheme::Stripe, SECRET, body, t, None)[0]
        .1
        .clone()
}

fn checkout_completed_event(sub_id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "evt_test",
        "type": "checkout.session.completed",
        "data": { "object": {
            "id": "cs_test",
            "customer": "cus_e2e",
            "subscription": sub_id,
            "metadata": { "plan": "pro" }
        }}
    })
}

#[test]
fn webhook_signature_accepts_valid_and_rejects_tamper_and_stale() {
    let body = br#"{"id":"evt_1","type":"checkout.session.completed"}"#;
    let now = 1_800_000_000;

    // valid
    let header = signed_header(body, now);
    assert!(verify_stripe_header(&header, body, SECRET, now).is_ok());

    // tampered body
    let header = signed_header(body, now);
    assert!(matches!(
        verify_stripe_header(&header, b"tampered", SECRET, now),
        Err(WebhookOutcome::InvalidSignature)
    ));

    // wrong secret
    let header = signed_header(body, now);
    assert!(matches!(
        verify_stripe_header(&header, body, "whsec_other", now),
        Err(WebhookOutcome::InvalidSignature)
    ));

    // stale beyond tolerance (300s)
    let header = signed_header(body, now - 301);
    assert!(matches!(
        verify_stripe_header(&header, body, SECRET, now),
        Err(WebhookOutcome::StaleTimestamp)
    ));

    // within tolerance is fine
    let header = signed_header(body, now - 299);
    assert!(verify_stripe_header(&header, body, SECRET, now).is_ok());
}

#[test]
fn subscription_transitions_drive_entitlements() {
    let db = Db::open(std::path::Path::new(":memory:")).unwrap();
    let sub = Mutex::new(Billing::default().subscription.into_inner());

    // start free
    assert_eq!(sub.lock().plan().id, FREE_PLAN);

    // upgrade via checkout.session.completed
    let out = apply_stripe_event(Some(&db), &sub, &checkout_completed_event("sub_123"));
    assert!(matches!(out, WebhookOutcome::Received));
    {
        let s = sub.lock();
        assert_eq!(s.plan_id, PRO_PLAN);
        assert_eq!(s.status, relay_engine::billing::SubStatus::Active);
        assert_eq!(s.stripe_customer_id.as_deref(), Some("cus_e2e"));
        assert!(s.plan().allows_channels(999));
    }

    // persisted: a fresh Db handle over the same file would read it back; here we
    // verify save/load through a second connection on the in-memory db is not
    // possible, so assert the save call succeeded by re-loading via the same db.
    let loaded = db
        .load_subscription()
        .unwrap()
        .expect("subscription persisted");
    assert_eq!(loaded.plan_id, PRO_PLAN);

    // provider-side cancellation downgrades
    let cancel = serde_json::json!({
        "type": "customer.subscription.deleted",
        "data": { "object": { "id": "sub_123" } }
    });
    apply_stripe_event(Some(&db), &sub, &cancel);
    let s = sub.lock();
    assert_eq!(s.plan_id, FREE_PLAN);
    assert_eq!(s.status, relay_engine::billing::SubStatus::Canceled);
    assert!(!s.entitled());
    assert_eq!(s.plan().max_channels, 5);
}

#[test]
fn past_due_keeps_plan_but_flags_status() {
    let sub = Mutex::new(Billing::default().subscription.into_inner());
    apply_stripe_event(None, &sub, &checkout_completed_event("sub_x"));
    let event = serde_json::json!({
        "type": "customer.subscription.updated",
        "data": { "object": { "status": "past_due", "current_period_end": 1900000000 } }
    });
    apply_stripe_event(None, &sub, &event);
    let s = sub.lock();
    assert_eq!(s.status, relay_engine::billing::SubStatus::PastDue);
    assert!(!s.entitled(), "past_due must not grant pro entitlements");
}

#[test]
fn unhandled_events_are_reported_not_fatal() {
    let sub = Mutex::new(Billing::default().subscription.into_inner());
    let event = serde_json::json!({ "type": "invoice.paid", "data": { "object": {} } });
    assert!(matches!(
        apply_stripe_event(None, &sub, &event),
        WebhookOutcome::Unhandled(_)
    ));
    assert_eq!(sub.lock().plan_id, FREE_PLAN);
}

#[tokio::test]
async fn quota_gates_channels_and_forward_rules_when_enabled() {
    let st = state_with_billing(true, None);
    // free plan: 5 channels max
    for i in 0..5 {
        let resp = relay_engine::control::router(st.clone())
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(format!("/api/channels/ch{i}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status() == axum::http::StatusCode::OK
                || resp.status() == axum::http::StatusCode::CREATED,
            "channel {i}: {}",
            resp.status()
        );
    }
    // 6th is rejected with 402 quota_exceeded
    let resp = relay_engine::control::router(st.clone())
        .oneshot(
            axum::http::Request::builder()
                .method("PUT")
                .uri("/api/channels/ch5")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::PAYMENT_REQUIRED);
    let bytes = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err["error"]["code"], "quota_exceeded");

    // updating an existing channel stays allowed
    let resp = relay_engine::control::router(st.clone())
        .oneshot(
            axum::http::Request::builder()
                .method("PUT")
                .uri("/api/channels/ch0")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // forward rules are a pro feature: rejected on free even within channel quota
    let rule = serde_json::json!({ "forward": [ { "url": "http://localhost:9/x" } ] });
    let resp = relay_engine::control::router(st.clone())
        .oneshot(
            axum::http::Request::builder()
                .method("PUT")
                .uri("/api/channels/ch0")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(rule.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::PAYMENT_REQUIRED);
}

#[tokio::test]
async fn upgrade_via_webhook_unlocks_quota() {
    let db = Db::open(std::path::Path::new(":memory:")).unwrap();
    let st = state_with_billing(true, Some(db.clone()));

    // fill the free quota
    for i in 0..5 {
        st.store
            .upsert(&format!("c{i}"), Default::default())
            .unwrap();
    }
    assert!(!st.billing.subscription.lock().plan().allows_channels(5));

    // Stripe confirms the upgrade (verified event applied through the real path)
    let body = serde_json::to_vec(&checkout_completed_event("sub_up")).unwrap();
    let header = signed_header(&body, relay_engine::envelope::now_utc().unix_timestamp());
    let out = relay_engine::control::router(st.clone())
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/billing/webhook")
                .header("stripe-signature", header)
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), axum::http::StatusCode::OK, "{:?}", {
        let b = http_body_util::BodyExt::collect(out.into_body())
            .await
            .unwrap()
            .to_bytes();
        String::from_utf8_lossy(&b).to_string()
    });

    assert_eq!(st.billing.subscription.lock().plan_id, PRO_PLAN);
    // pro: unlimited channels
    st.store.upsert("c5", Default::default()).unwrap();
    st.store.upsert("c6", Default::default()).unwrap();

    // subscription persisted for restart
    assert_eq!(db.load_subscription().unwrap().unwrap().plan_id, PRO_PLAN);
}

#[tokio::test]
async fn webhook_without_signature_secret_is_422() {
    // Engine with no webhook secret configured anywhere (config nor ambient env).
    // Ambient shells often export STRIPE_WEBHOOK_SECRET; clear it around this test.
    // No other test reads this env var (they use the config field), so no lock is
    // needed despite running in parallel.
    let prior = std::env::var("STRIPE_WEBHOOK_SECRET").ok();
    std::env::remove_var("STRIPE_WEBHOOK_SECRET");
    let st = state_with_billing_and_secret(true, None, None);
    let out = relay_engine::control::router(st)
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/billing/webhook")
                .body(axum::body::Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    if let Some(v) = prior {
        std::env::set_var("STRIPE_WEBHOOK_SECRET", v);
    }
}

#[tokio::test]
async fn plans_endpoint_lists_catalog_and_subscription() {
    let st = state_with_billing(true, None);
    let out = relay_engine::control::router(st)
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/billing/plans")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.status(), axum::http::StatusCode::OK);
    let bytes = http_body_util::BodyExt::collect(out.into_body())
        .await
        .unwrap()
        .to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["billing_enabled"], true);
    assert_eq!(v["plans"].as_array().unwrap().len(), PLANS.len());
    assert_eq!(v["subscription"]["plan_id"], FREE_PLAN);
}

#[test]
fn checkout_form_carries_plan_metadata() {
    let form = build_checkout_form("price_123", "https://app/success", "https://app/cancel");
    assert!(form.contains(&("mode".into(), "subscription".into())));
    assert!(form.contains(&("line_items[0][price]".into(), "price_123".into())));
    assert!(form.contains(&("metadata[plan]".into(), PRO_PLAN.into())));
}

#[test]
fn plan_catalog_is_sane() {
    assert_eq!(PLANS.len(), 2);
    assert!(PLANS[0].max_channels < PLANS[1].max_channels);
    assert!(plan_by_id("nope").is_none());
}
