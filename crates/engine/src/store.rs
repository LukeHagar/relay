//! Channels, configs, per-channel sequence numbers and ring buffers (§3, §4).

use crate::envelope::{build_envelope, CaptureInput, Envelope};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use time::OffsetDateTime;

pub fn valid_channel_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub const RESERVED_SLUGS: &[&str] = &[
    "api", "app", "www", "admin", "relay", "capture", "mail", "ftp", "ns1", "ns2",
];

pub fn valid_slug(slug: &str) -> bool {
    (3..=63).contains(&slug.len())
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !RESERVED_SLUGS.contains(&slug)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseMode {
    Ack,
    Echo,
    Static,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BufferConfig {
    pub max_events: usize,
    pub max_age_s: i64,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_events: 1000,
            max_age_s: 86_400,
        }
    }
}

/// Optional capture-plane protection (§9.7): require `Authorization: Bearer <env value>`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthConfig {
    pub bearer_env: String,
}

/// Per-channel capture rate limit (§9.7). Defaults to 300 burst / 100 rps refill.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RateLimitConfig {
    pub burst: u32,
    pub refill_per_s: u32,
}

/// One auto-forward rule on a channel (M3): every captured event is re-delivered
/// to `url` (source path appended), optionally signed via a named profile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForwardRule {
    pub url: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChannelConfig {
    #[serde(default)]
    pub response_mode: Option<ResponseMode>,
    #[serde(default)]
    pub static_response: Option<StaticResponse>,
    #[serde(default)]
    pub buffer: Option<BufferConfig>,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    /// Auto-forward rules (M3): capture fans out to N targets.
    #[serde(default)]
    pub forward: Vec<ForwardRule>,
    /// Capture-plane rate limit override (§8/§9.7).
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

impl ChannelConfig {
    pub fn response_mode(&self) -> ResponseMode {
        self.response_mode.unwrap_or(ResponseMode::Ack)
    }
    pub fn buffer(&self) -> BufferConfig {
        self.buffer.unwrap_or_default()
    }
    pub fn rate_limit(&self) -> RateLimitConfig {
        self.rate_limit.unwrap_or(RateLimitConfig {
            burst: 300,
            refill_per_s: 100,
        })
    }
}

struct Inner {
    next_seq: u64,
    buffer: VecDeque<BufferEntry>,
}

pub struct BufferEntry {
    pub event: Arc<Envelope>,
    pub at: OffsetDateTime,
}

/// Per-channel token bucket (§9.7): default 300 burst, 100 events/s refill.
pub struct RateLimiter {
    burst: f64,
    refill_per_s: f64,
    state: Mutex<(f64, std::time::Instant)>,
}

impl RateLimiter {
    fn from_config(cfg: RateLimitConfig) -> Self {
        Self {
            burst: cfg.burst as f64,
            refill_per_s: cfg.refill_per_s as f64,
            state: Mutex::new((cfg.burst as f64, std::time::Instant::now())),
        }
    }

    pub fn allow(&self) -> bool {
        let mut st = self.state.lock();
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(st.1).as_secs_f64();
        st.0 = (st.0 + elapsed * self.refill_per_s).min(self.burst);
        st.1 = now;
        if st.0 >= 1.0 {
            st.0 -= 1.0;
            true
        } else {
            false
        }
    }
}

pub struct Channel {
    pub name: String,
    pub created_at: String,
    /// Arc-swap: the capture hot path takes a refcount instead of deep-cloning
    /// the whole config (incl. the forward Vec) on every request.
    config: RwLock<Arc<ChannelConfig>>,
    inner: Mutex<Inner>,
    rate: RwLock<Arc<RateLimiter>>,
}

impl Channel {
    fn new(name: String, config: ChannelConfig, created_at: String, start_next_seq: u64) -> Self {
        let rate = Arc::new(RateLimiter::from_config(config.rate_limit()));
        Self {
            name,
            created_at,
            config: RwLock::new(Arc::new(config)),
            inner: Mutex::new(Inner {
                next_seq: start_next_seq,
                buffer: VecDeque::new(),
            }),
            rate: RwLock::new(rate),
        }
    }

    pub fn config(&self) -> Arc<ChannelConfig> {
        self.config.read().clone()
    }

    pub fn update_config(&self, patch: ChannelConfig) {
        let old_limit = self.config.read().rate_limit();
        let limit_changed = patch.rate_limit.map(|r| (r.burst, r.refill_per_s))
            != Some((old_limit.burst, old_limit.refill_per_s));
        *self.config.write() = Arc::new(patch);
        if limit_changed {
            *self.rate.write() =
                Arc::new(RateLimiter::from_config(self.config.read().rate_limit()));
        }
    }

    /// Snapshot the active limiter (capture plane calls `allow` through it).
    pub fn rate_limiter(&self) -> Arc<RateLimiter> {
        self.rate.read().clone()
    }

    /// Mint the next envelope for this channel under the sequence lock and append it
    /// to the ring buffer. Returns the shared, immutable event.
    pub fn push(&self, input: CaptureInput, now: OffsetDateTime) -> Arc<Envelope> {
        let cfg = self.config();
        let mut inner = self.inner.lock();
        inner.next_seq += 1;
        let seq = inner.next_seq;
        let event = Arc::new(build_envelope(&input, seq, now));
        let cutoff = now - time::Duration::seconds(cfg.buffer().max_age_s);
        while let Some(front) = inner.buffer.front() {
            if front.at < cutoff || inner.buffer.len() >= cfg.buffer().max_events {
                inner.buffer.pop_front();
            } else {
                break;
            }
        }
        inner.buffer.push_back(BufferEntry {
            event: event.clone(),
            at: now,
        });
        event
    }

    /// Newest `seq` allocated so far (0 for a fresh channel).
    pub fn last_seq(&self) -> u64 {
        self.inner.lock().next_seq
    }

    /// History ascending, strictly above `cursor`, capped at `limit`.
    pub fn history(&self, cursor: Option<u64>, limit: usize) -> Vec<Arc<Envelope>> {
        let inner = self.inner.lock();
        inner
            .buffer
            .iter()
            .filter(|e| cursor.is_none_or(|c| e.event.seq > c))
            .take(limit)
            .map(|e| e.event.clone())
            .collect()
    }

    /// Backlog snapshot + seam position for a new subscription (§5.1 delivery rules).
    pub fn snapshot_from(&self, from: &FromSpec) -> (Vec<Arc<Envelope>>, u64) {
        let inner = self.inner.lock();
        let top = inner.next_seq;
        match from {
            FromSpec::Newest => (Vec::new(), top),
            FromSpec::Oldest => (inner.buffer.iter().map(|e| e.event.clone()).collect(), top),
            FromSpec::At(n) => (
                inner
                    .buffer
                    .iter()
                    .filter(|e| e.event.seq >= *n)
                    .map(|e| e.event.clone())
                    .collect(),
                top,
            ),
        }
    }
}

/// Subscription start point for backlog replay (§5.1): `"newest"`, `"oldest"`, or a seq.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromSpec {
    Newest,
    Oldest,
    At(u64),
}

impl FromSpec {
    pub fn parse(v: serde_json::Value) -> Option<Self> {
        match v {
            serde_json::Value::String(s) => match s.as_str() {
                "newest" => Some(FromSpec::Newest),
                "oldest" => Some(FromSpec::Oldest),
                _ => None,
            },
            serde_json::Value::Number(n) => n.as_u64().map(FromSpec::At),
            _ => None,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            FromSpec::Newest => "newest",
            FromSpec::Oldest => "oldest",
            FromSpec::At(_) => "seq",
        }
    }
}

#[derive(Default)]
pub struct Store {
    channels: RwLock<HashMap<String, Arc<Channel>>>,
}

pub struct UpsertResult {
    pub channel: Arc<Channel>,
    pub created: bool,
}

impl Store {
    pub fn upsert(&self, name: &str, config: ChannelConfig) -> anyhow::Result<UpsertResult> {
        if !valid_channel_name(name) {
            anyhow::bail!("invalid channel name");
        }
        if let Some(sr) = &config.static_response {
            if config.response_mode == Some(ResponseMode::Static)
                && !(100..600).contains(&sr.status)
            {
                anyhow::bail!("static_response.status out of range");
            }
        }
        let mut map = self.channels.write();
        if let Some(existing) = map.get(name) {
            existing.update_config(config);
            return Ok(UpsertResult {
                channel: existing.clone(),
                created: false,
            });
        }
        let channel = Arc::new(Channel::new(
            name.to_string(),
            config,
            crate::envelope::format_rfc3339_ms(crate::envelope::truncate_ms(
                crate::envelope::now_utc(),
            )),
            0,
        ));
        map.insert(name.to_string(), channel.clone());
        Ok(UpsertResult {
            channel,
            created: true,
        })
    }

    /// Boot-time restore from the durable layer: keeps `created_at` and continues
    /// `seq` above the highest persisted event (`last_allocated_seq`) so cursors
    /// survive restarts.
    pub fn restore(
        &self,
        name: &str,
        config: ChannelConfig,
        created_at: String,
        next_seq: u64,
    ) -> anyhow::Result<Arc<Channel>> {
        if !valid_channel_name(name) {
            anyhow::bail!("invalid channel name");
        }
        let mut map = self.channels.write();
        if let Some(existing) = map.get(name) {
            return Ok(existing.clone());
        }
        let channel = Arc::new(Channel::new(name.to_string(), config, created_at, next_seq));
        map.insert(name.to_string(), channel.clone());
        Ok(channel)
    }

    pub fn get(&self, name: &str) -> Option<Arc<Channel>> {
        self.channels.read().get(name).cloned()
    }

    pub fn remove(&self, name: &str) -> Option<Arc<Channel>> {
        self.channels.write().remove(name)
    }

    pub fn list(&self) -> Vec<Arc<Channel>> {
        let mut v: Vec<_> = self.channels.read().values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}

/// Extract a valid slug from a `Host` for hosted capture routing (§3.1).
pub fn slug_of_host(host: &str, zone: &str) -> Option<String> {
    let host = host.split(':').next()?;
    let suffix = format!(".{zone}");
    let slug = host.strip_suffix(&suffix)?;
    if valid_slug(slug) {
        Some(slug.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    fn input(channel: &str, body: &str) -> CaptureInput {
        CaptureInput {
            channel: channel.into(),
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

    #[test]
    fn name_and_slug_validation() {
        assert!(valid_channel_name("github"));
        assert!(valid_channel_name("a-b_c9"));
        assert!(!valid_channel_name(""));
        assert!(!valid_channel_name("has space"));
        assert!(!valid_channel_name(&"x".repeat(65)));

        assert!(valid_slug("quiet-river-42"));
        assert!(!valid_slug("ab")); // < 3 chars
        assert!(!valid_slug("UPPER"));
        assert!(!valid_slug("api")); // reserved
    }

    #[test]
    fn upsert_is_idempotent_and_updates_config() {
        let store = Store::default();
        let r1 = store.upsert("gh", ChannelConfig::default()).unwrap();
        assert!(r1.created);
        let r2 = store
            .upsert(
                "gh",
                ChannelConfig {
                    response_mode: Some(ResponseMode::Echo),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!r2.created);
        assert_eq!(r2.channel.config().response_mode(), ResponseMode::Echo);
    }

    #[test]
    fn sequences_are_gap_free_and_monotonic() {
        let store = Store::default();
        let ch = store.upsert("s", ChannelConfig::default()).unwrap().channel;
        for i in 1..=5 {
            let env = ch.push(input("s", &i.to_string()), crate::envelope::now_utc());
            assert_eq!(env.seq, i);
        }
        assert_eq!(ch.last_seq(), 5);
        let seqs: Vec<u64> = ch.history(None, 100).iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn ring_buffer_evicts_by_count() {
        let store = Store::default();
        let ch = store
            .upsert(
                "tiny",
                ChannelConfig {
                    buffer: Some(BufferConfig {
                        max_events: 3,
                        max_age_s: 3600,
                    }),
                    ..Default::default()
                },
            )
            .unwrap()
            .channel;
        for i in 1..=6 {
            ch.push(input("tiny", &i.to_string()), crate::envelope::now_utc());
        }
        let seqs: Vec<u64> = ch.history(None, 100).iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![4, 5, 6]);
    }

    #[test]
    fn ring_buffer_evicts_by_age() {
        let store = Store::default();
        let ch = store
            .upsert(
                "old",
                ChannelConfig {
                    buffer: Some(BufferConfig {
                        max_events: 100,
                        max_age_s: 10,
                    }),
                    ..Default::default()
                },
            )
            .unwrap()
            .channel;
        let past = crate::envelope::now_utc() - time::Duration::seconds(60);
        ch.push(input("old", "ancient"), past);
        ch.push(input("old", "fresh"), crate::envelope::now_utc());
        let seqs: Vec<u64> = ch.history(None, 100).iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![2]);
    }

    #[test]
    fn history_cursor_is_exclusive() {
        let store = Store::default();
        let ch = store.upsert("c", ChannelConfig::default()).unwrap().channel;
        for _ in 1..=4 {
            ch.push(input("c", ""), crate::envelope::now_utc());
        }
        let seqs: Vec<u64> = ch.history(Some(2), 100).iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![3, 4]);
        assert!(ch.history(Some(4), 100).is_empty());
    }

    #[test]
    fn rate_limiter_bursts_then_refills() {
        static RL: LazyLock<RateLimiter> = LazyLock::new(|| RateLimiter {
            burst: 3.0,
            refill_per_s: 1000.0,
            state: Mutex::new((3.0, std::time::Instant::now())),
        });
        assert!(RL.allow());
        assert!(RL.allow());
        assert!(RL.allow());
        assert!(!RL.allow()); // burst exhausted
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(RL.allow()); // refilled
    }

    #[test]
    fn snapshot_seams_have_no_gaps_or_duplicates() {
        let store = Store::default();
        let ch = store
            .upsert("seam", ChannelConfig::default())
            .unwrap()
            .channel;
        for _ in 1..=3 {
            ch.push(input("seam", ""), crate::envelope::now_utc());
        }
        // oldest: everything, live resumes above top
        let (backlog, seam) =
            ch.snapshot_from(&FromSpec::parse(serde_json::json!("oldest")).unwrap());
        assert_eq!(backlog.len(), 3);
        assert_eq!(seam, 3);
        // newest: nothing, live resumes above top
        let (backlog, seam) =
            ch.snapshot_from(&FromSpec::parse(serde_json::json!("newest")).unwrap());
        assert!(backlog.is_empty());
        assert_eq!(seam, 3);
        // at 2: seqs 2..3, live resumes above top
        let (backlog, seam) = ch.snapshot_from(&FromSpec::parse(serde_json::json!(2)).unwrap());
        let seqs: Vec<u64> = backlog.iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![2, 3]);
        assert_eq!(seam, 3);
    }

    #[tokio::test]
    async fn custom_rate_limits_apply_and_update() {
        use crate::capture;
        use crate::config::Config;
        use crate::hub::Hub;
        use tower::ServiceExt;

        let st = crate::AppState {
            config: Config::default(),
            store: Arc::new(Store::default()),
            hub: Arc::new(Hub::default()),
            profiles: Arc::new(crate::signing::Profiles::default()),
            templates: Arc::new(
                crate::templates::Registry::load(crate::BUNDLED_TEMPLATES, None).unwrap(),
            ),
            http: reqwest::Client::new(),
            db: None,
            billing: Arc::new(crate::billing::Billing::default()),
            persisted_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
        // generous limit
        st.store
            .upsert(
                "fast",
                ChannelConfig {
                    rate_limit: Some(RateLimitConfig {
                        burst: 5000,
                        refill_per_s: 5000,
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        st.store.upsert("slow", ChannelConfig::default()).unwrap();

        async fn post(router: axum::Router, uri: &str) -> http::StatusCode {
            router
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(uri)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
                .status()
        }

        let router = capture::router(st.clone());
        for _ in 0..400 {
            assert_eq!(
                post(router.clone(), "/c/fast/x").await,
                http::StatusCode::OK
            );
        } // no 429 under the raised ceiling

        // lower it via config update — next burst is throttled again
        st.store.get("fast").unwrap().update_config(ChannelConfig {
            rate_limit: Some(RateLimitConfig {
                burst: 2,
                refill_per_s: 1,
            }),
            ..Default::default()
        });
        let codes: Vec<http::StatusCode> =
            futures::future::join_all((0..10).map(|_| post(router.clone(), "/c/fast/y"))).await;
        assert!(codes.contains(&http::StatusCode::TOO_MANY_REQUESTS));
    }

    #[test]
    fn slug_extraction_from_hosts() {
        assert_eq!(
            slug_of_host(
                "github.acme.capture.example.com",
                "acme.capture.example.com"
            ),
            Some("github".to_string())
        );
        assert_eq!(
            slug_of_host(
                "github.acme.capture.example.com:443",
                "acme.capture.example.com"
            ),
            Some("github".to_string())
        );
        assert_eq!(
            slug_of_host("api.acme.capture.example.com", "acme.capture.example.com"),
            None
        );
        assert_eq!(
            slug_of_host("other.example.com", "acme.capture.example.com"),
            None
        );
    }
}
