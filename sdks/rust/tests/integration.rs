//! Integration tests for the Rust SDK — skipped unless RELAY_TEST_URL points at a
//! running engine (`relay serve`).

use relay_sdk::{ChannelConfig, FromSpec, RelayClient};

fn client() -> Option<RelayClient> {
    let base = std::env::var("RELAY_TEST_URL").ok()?;
    Some(RelayClient::new(&base).unwrap())
}

#[tokio::test]
async fn channel_capture_history_round_trip() {
    let Some(relay) = client() else { return };
    relay
        .upsert_channel("sdk-rust", &ChannelConfig::default())
        .await
        .unwrap();

    // produce through the capture plane (default port mapping :8000 → :8001)
    let capture = relay.capture_url("sdk-rust");
    reqwest::Client::new()
        .post(format!("{capture}/evt1?a=1"))
        .header("content-type", "application/json")
        .body(r#"{"hello":"rust-sdk"}"#)
        .send()
        .await
        .unwrap();

    let events = relay.history("sdk-rust", None, 100).await.unwrap();
    assert!(!events.is_empty(), "expected history");
    let env = events.last().unwrap();
    assert_eq!(env.query.as_deref(), Some("a=1"));
    assert!(env.body.contains("rust-sdk"));
}

#[tokio::test]
async fn once_receives_live_event_after_handshake() {
    let Some(relay) = client() else { return };
    relay
        .upsert_channel("sdk-rust-once", &ChannelConfig::default())
        .await
        .unwrap();

    // produce runs only after the engine confirms the subscription, so the
    // delivery cannot race the handshake.
    let env = relay
        .once(
            "sdk-rust-once",
            FromSpec::Newest,
            Some(|| {
                tokio::spawn(async move {
                    reqwest::Client::new()
                        .post("http://127.0.0.1:8001/c/sdk-rust-once/live")
                        .body("ping")
                        .send()
                        .await
                        .unwrap();
                });
            }),
        )
        .await
        .unwrap();

    assert_eq!(env.path, "/live");
    assert_eq!(env.body, "ping");
}

#[tokio::test]
async fn replay_typed_errors_surface() {
    let Some(relay) = client() else { return };
    use relay_sdk::{ReplayRequest, ReplaySourceWire};
    let err = relay
        .replay(&ReplayRequest {
            target_url: "http://127.0.0.1:8001".into(),
            source: ReplaySourceWire::Event {
                event_id: "01NOPE".into(),
            },
            ..ReplayRequest::default()
        })
        .await
        .unwrap_err();
    assert_eq!(err.code, "no_such_event");
}
