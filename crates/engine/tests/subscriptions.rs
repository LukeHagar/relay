//! End-to-end subscription tests over real sockets (§5).

use futures::{SinkExt, StreamExt};
use relay_engine::store::{ChannelConfig, Store};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

fn test_state() -> relay_engine::AppState {
    relay_engine::AppState {
        config: relay_engine::config::Config::default(),
        store: Arc::new(Store::default()),
        hub: Arc::new(relay_engine::hub::Hub::default()),
        profiles: Arc::new(relay_engine::signing::Profiles::default()),
        templates: Arc::new(
            relay_engine::templates::Registry::load(relay_engine::BUNDLED_TEMPLATES, None).unwrap(),
        ),
        http: reqwest::Client::new(),
        db: None,
        persisted_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        billing: Arc::new(relay_engine::billing::Billing::default()),
    }
}

async fn spawn_control(state: relay_engine::AppState) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, relay_engine::control::router(state))
            .await
            .unwrap();
    });
    format!("ws://{addr}")
}

async fn ws_connect(
    url: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    ws
}

fn capture(channel: &str, body: &str, path: &str) -> relay_engine::envelope::CaptureInput {
    relay_engine::envelope::CaptureInput {
        channel: channel.into(),
        host: None,
        method: "POST".into(),
        raw_path: path.into(),
        raw_query: None,
        headers: vec![],
        body: bytes::Bytes::copy_from_slice(body.as_bytes()),
        truncated: false,
        status_sent: 200,
        remote_addr: None,
    }
}

#[tokio::test]
async fn ws_backlog_then_live_with_ping_and_close() {
    let state = test_state();
    let ch = state
        .store
        .upsert("gh", ChannelConfig::default())
        .unwrap()
        .channel;
    for i in 1..=3 {
        let env = ch.push(
            capture("gh", &i.to_string(), "/"),
            relay_engine::envelope::now_utc(),
        );
        assert_eq!(env.seq, i);
    }
    let base = spawn_control(state.clone()).await;
    let mut ws = ws_connect(&format!("{base}/ws")).await;

    // subscribe from seq 2 → ok{last_seq:3}, then backlog 2,3
    ws.send(Message::Text(
        serde_json::json!({"type":"subscribe","id":"s1","channel":"gh","from":2})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();

    let ok = match ws.next().await.unwrap().unwrap() {
        Message::Text(t) => serde_json::from_str::<serde_json::Value>(&t).unwrap(),
        m => panic!("unexpected {m:?}"),
    };
    assert_eq!(ok["type"], "ok");
    assert_eq!(ok["id"], "s1");
    assert_eq!(ok["last_seq"], 3);

    for want in [2u64, 3] {
        match ws.next().await.unwrap().unwrap() {
            Message::Text(t) => {
                let v: serde_json::Value = serde_json::from_str(&t).unwrap();
                assert_eq!(v["type"], "event");
                assert_eq!(v["subscription"], "s1");
                assert_eq!(v["event"]["seq"], want);
            }
            m => panic!("unexpected {m:?}"),
        }
    }

    // live delivery exactly once at the seam
    let env4 = ch.push(capture("gh", "4", "/"), relay_engine::envelope::now_utc());
    state.hub.publish(&env4);
    match ws.next().await.unwrap().unwrap() {
        Message::Text(t) => {
            let v: serde_json::Value = serde_json::from_str(&t).unwrap();
            assert_eq!(v["event"]["seq"], 4);
        }
        m => panic!("unexpected {m:?}"),
    }

    // app-level ping/pong
    ws.send(Message::Text(r#"{"type":"ping","id":"p1"}"#.into()))
        .await
        .unwrap();
    loop {
        match ws.next().await.unwrap().unwrap() {
            Message::Text(t) => {
                let v: serde_json::Value = serde_json::from_str(&t).unwrap();
                if v["type"] == "pong" {
                    assert_eq!(v["id"], "p1");
                    break;
                }
            }
            m => panic!("unexpected {m:?}"),
        }
    }

    // subscribe to a missing channel → error frame, connection stays up
    ws.send(Message::Text(
        r#"{"type":"subscribe","id":"s2","channel":"ghost","from":"newest"}"#.into(),
    ))
    .await
    .unwrap();
    match ws.next().await.unwrap().unwrap() {
        Message::Text(t) => {
            let v: serde_json::Value = serde_json::from_str(&t).unwrap();
            assert_eq!(v["type"], "error");
            assert_eq!(v["error"]["code"], "no_such_channel");
        }
        m => panic!("unexpected {m:?}"),
    }

    ws.close(None).await.unwrap();
}

#[tokio::test]
async fn sse_streams_backlog_then_live() {
    let state = test_state();
    let ch = state
        .store
        .upsert("sse-ch", ChannelConfig::default())
        .unwrap()
        .channel;
    for i in 1..=2 {
        ch.push(
            capture("sse-ch", &i.to_string(), "/x"),
            relay_engine::envelope::now_utc(),
        );
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, relay_engine::control::router(state.clone()))
            .await
            .unwrap();
        let _ = ch; // keep channel alive for closure clarity
    });

    // connect with Last-Event-ID absent + from=oldest via query

    let resp = reqwest::get(format!("http://{addr}/api/channels/sse-ch/sse?from=oldest"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));

    // read the two backlog frames incrementally
    let mut stream = resp.bytes_stream();
    let mut buf = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while buf.iter().filter(|b| **b == b'\n').count() < 8 && std::time::Instant::now() < deadline {
        if let Ok(Some(chunk)) =
            tokio::time::timeout(std::time::Duration::from_secs(2), stream.next()).await
        {
            buf.extend_from_slice(&chunk.unwrap());
        }
    }
    let text = String::from_utf8_lossy(&buf);
    assert_eq!(text.matches("event: relay.event").count(), 2);
    assert!(text.contains("id: 1"));
    assert!(text.contains("id: 2"));
}
