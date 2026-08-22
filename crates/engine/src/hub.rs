//! Subscription hub (§5): fan-out to WebSocket/SSE subscribers with per-subscription
//! buffering, seq-seam dedup, and slow-consumer disconnects.

use crate::envelope::Envelope;
use crate::store::Channel;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Per-subscription outbound buffer (§8, fixed for v1).
pub const SUBSCRIPTION_BUFFER: usize = 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

/// Server → client frames. Every frame carries `"v": 1` and a `type` tag (§5.1).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    Ok {
        v: u8,
        id: String,
        last_seq: u64,
    },
    Error {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        error: ApiError,
    },
    Event {
        v: u8,
        subscription: String,
        event: Arc<Envelope>,
    },
    Pong {
        v: u8,
        id: String,
    },
    ChannelClosed {
        v: u8,
        channel: String,
    },
}

pub struct Subscription {
    pub corr_id: String,
    pub channel: String,
    pub tx: mpsc::Sender<ServerFrame>,
    /// Live delivery resumes strictly above this seq (no gaps, no duplicates at the seam).
    pub last_sent: u64,
}

impl ServerFrame {
    const V: u8 = 1;

    pub(crate) fn ok(id: impl Into<String>, last_seq: u64) -> Self {
        Self::Ok {
            v: Self::V,
            id: id.into(),
            last_seq,
        }
    }
    pub(crate) fn error(id: Option<String>, code: &str, message: impl Into<String>) -> Self {
        Self::Error {
            v: Self::V,
            id,
            error: ApiError::new(code, message),
        }
    }
    pub(crate) fn event(subscription: impl Into<String>, event: Arc<Envelope>) -> Self {
        Self::Event {
            v: Self::V,
            subscription: subscription.into(),
            event,
        }
    }
    pub(crate) fn pong(id: impl Into<String>) -> Self {
        Self::Pong {
            v: Self::V,
            id: id.into(),
        }
    }
    pub(crate) fn channel_closed(channel: impl Into<String>) -> Self {
        Self::ChannelClosed {
            v: Self::V,
            channel: channel.into(),
        }
    }
}

#[derive(Default)]
pub struct Hub {
    subs: Mutex<Vec<Subscription>>,
    /// Fast-path counter so captures on idle channels skip the subscription lock.
    sub_count: std::sync::atomic::AtomicUsize,
}

impl Hub {
    pub fn subscribe(
        &self,
        channel: &Arc<Channel>,
        corr_id: &str,
        from: &crate::store::FromSpec,
        tx: mpsc::Sender<ServerFrame>,
    ) -> Vec<Arc<Envelope>> {
        let (backlog, seam) = channel.snapshot_from(from);
        self.subs.lock().push(Subscription {
            corr_id: corr_id.to_string(),
            channel: channel.name.clone(),
            tx,
            last_sent: seam,
        });
        self.sub_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        backlog
    }

    pub fn unsubscribe(&self, corr_id: &str, channel: &str) -> bool {
        let mut subs = self.subs.lock();
        let before = subs.len();
        subs.retain(|s| !(s.corr_id == corr_id && s.channel == channel));
        let removed = before != subs.len();
        if removed {
            self.sub_count
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
        removed
    }

    pub fn subscriber_count(&self, channel: &str) -> usize {
        self.subs
            .lock()
            .iter()
            .filter(|s| s.channel == channel)
            .count()
    }

    /// Deliver one event to every matching subscription. Overflowing a subscription's
    /// buffer removes it; the writer is told via an `error` frame, then the stream ends
    /// (client reconnects with a cursor — §5.1 delivery semantics item 4).
    pub fn publish(&self, event: &Arc<Envelope>) {
        if self.sub_count.load(std::sync::atomic::Ordering::Relaxed) == 0 {
            return; // no subscribers anywhere: skip the lock entirely
        }
        let mut overflowed: Vec<(mpsc::Sender<ServerFrame>, String)> = Vec::new();
        {
            let mut subs = self.subs.lock();
            subs.retain_mut(|s| {
                if s.channel != event.channel || event.seq <= s.last_sent {
                    return true;
                }
                match s
                    .tx
                    .try_send(ServerFrame::event(s.corr_id.clone(), event.clone()))
                {
                    Ok(()) => {
                        s.last_sent = event.seq;
                        true
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        overflowed.push((s.tx.clone(), s.corr_id.clone()));
                        false
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        self.sub_count
                            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        false
                    }
                }
            });
        }
        for (tx, corr_id) in overflowed {
            tokio::spawn(async move {
                let _ = tx
                    .send(ServerFrame::error(
                        Some(corr_id),
                        "slow_consumer",
                        "subscription buffer overflowed; reconnect with a cursor",
                    ))
                    .await;
                // tx dropped here → writer observes end-of-stream and closes the socket
            });
        }
    }

    /// A channel was deleted: drop its subscriptions and notify their writers (§3.4).
    pub fn close_channel(&self, channel: &str) {
        let mut closed: Vec<mpsc::Sender<ServerFrame>> = Vec::new();
        {
            let mut subs = self.subs.lock();
            subs.retain(|s| {
                if s.channel == channel {
                    closed.push(s.tx.clone());
                    self.sub_count
                        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    false
                } else {
                    true
                }
            });
        }
        for tx in closed {
            let frame = ServerFrame::channel_closed(channel.to_string());
            tokio::spawn(async move {
                let _ = tx.send(frame).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::CaptureInput;
    use crate::store::{ChannelConfig, FromSpec, Store};

    fn input(body: &str) -> CaptureInput {
        CaptureInput {
            channel: "t".into(),
            host: None,
            method: "POST".into(),
            raw_path: "/".into(),
            raw_query: None,
            headers: vec![],
            body: bytes::Bytes::copy_from_slice(body.as_bytes()),
            truncated: false,
            status_sent: 200,
            remote_addr: None,
        }
    }

    fn frame_json(f: &ServerFrame) -> serde_json::Value {
        serde_json::to_value(f).unwrap()
    }

    #[tokio::test]
    async fn frames_serialize_with_contract_tags() {
        let env = Arc::new(crate::envelope::build_envelope(
            &input("x"),
            1,
            crate::envelope::now_utc(),
        ));
        let ok = frame_json(&ServerFrame::ok("c1".to_string(), 3u64));
        assert_eq!(ok["type"], "ok");
        assert_eq!(ok["last_seq"], 3);
        let ev = frame_json(&ServerFrame::event("c1".to_string(), env));
        assert_eq!(ev["type"], "event");
        assert_eq!(ev["event"]["v"], 1);
        let err = frame_json(&ServerFrame::error(None, "slow_consumer", "boom"));
        assert_eq!(err["type"], "error");
        assert_eq!(err["error"]["code"], "slow_consumer");
        assert!(err.get("id").is_none()); // omitted, not null
        let cc = frame_json(&ServerFrame::channel_closed("x".to_string()));
        assert_eq!(cc["type"], "channel_closed");
    }

    #[tokio::test]
    async fn publish_respects_seam_and_ordering() {
        let store = Store::default();
        let ch = store.upsert("t", ChannelConfig::default()).unwrap().channel;
        for i in 1..=3 {
            ch.push(input(&i.to_string()), crate::envelope::now_utc());
        }
        let hub = Hub::default();
        let (tx, mut rx) = mpsc::channel(SUBSCRIPTION_BUFFER);

        // subscribe from seq 2 → backlog 2,3 then live 4+
        let backlog = hub.subscribe(
            &ch,
            "s1",
            &FromSpec::parse(serde_json::json!(2)).unwrap(),
            tx.clone(),
        );
        let seqs: Vec<u64> = backlog.iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![2, 3]);
        // The transport (WS/SSE handler) writes the returned backlog first; live
        // delivery then flows through tx strictly above the seam.
        for e in &backlog {
            tx.send(ServerFrame::event("s1".to_string(), e.clone()))
                .await
                .unwrap();
        }

        ch.push(input("4"), crate::envelope::now_utc());
        let env4 = ch.history(Some(3), 1).pop().unwrap();
        hub.publish(&env4);

        // replayed backlog first
        for want in [2u64, 3] {
            match rx.recv().await.unwrap() {
                ServerFrame::Event { event, .. } => assert_eq!(event.seq, want),
                f => panic!("unexpected frame {f:?}"),
            }
        }
        // live 4 arrives exactly once — no duplicate of the seam
        match rx.recv().await.unwrap() {
            ServerFrame::Event { event, .. } => assert_eq!(event.seq, 4),
            f => panic!("unexpected frame {f:?}"),
        }
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn slow_consumer_is_disconnected_with_error_frame() {
        let store = Store::default();
        let ch = store.upsert("t", ChannelConfig::default()).unwrap().channel;
        let hub = Hub::default();
        let (tx, mut rx) = mpsc::channel(2); // tiny buffer to force overflow fast
        hub.subscribe(
            &ch,
            "s",
            &FromSpec::parse(serde_json::json!("newest")).unwrap(),
            tx,
        );

        for i in 0..10 {
            let env = ch.push(input(&i.to_string()), crate::envelope::now_utc());
            hub.publish(&env);
        }
        assert_eq!(hub.subscriber_count("t"), 0); // removed on overflow

        // writer drains what fit, then receives the slow_consumer error, then EOF
        let mut saw_slow = false;
        while let Some(f) = rx.recv().await {
            if let ServerFrame::Error { error, .. } = f {
                assert_eq!(error.code, "slow_consumer");
                saw_slow = true;
            }
        }
        assert!(saw_slow);
    }

    #[tokio::test]
    async fn close_channel_notifies_subscribers() {
        let store = Store::default();
        let ch = store.upsert("t", ChannelConfig::default()).unwrap().channel;
        let hub = Hub::default();
        let (tx, mut rx) = mpsc::channel(4);
        hub.subscribe(
            &ch,
            "s",
            &FromSpec::parse(serde_json::json!("newest")).unwrap(),
            tx,
        );
        hub.close_channel("t");
        assert_eq!(hub.subscriber_count("t"), 0);
        match rx.recv().await.unwrap() {
            ServerFrame::ChannelClosed { channel, .. } => assert_eq!(channel, "t"),
            f => panic!("unexpected frame {f:?}"),
        }
    }
}
