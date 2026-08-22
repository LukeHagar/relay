//! Relay Rust SDK — typed async client over the wire contract (docs/api.md).
//!
//! ```no_run
//! use relay_sdk::{FromSpec, RelayClient};
//!
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let relay = RelayClient::new("http://127.0.0.1:8000")?;
//! relay.upsert_channel("stripe-dev", &Default::default()).await?;
//! let env = relay
//!     .once("stripe-dev", FromSpec::Newest, Some(|| {
//!         // produce a delivery after the handshake confirms
//!     }))
//!     .await?;
//! println!("got {} {}", env.method, env.path);
//! # Ok(())
//! # }
//! ```

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Event envelope (§4). Field names match the wire format.
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    pub v: u8,
    pub id: String,
    pub channel: String,
    #[serde(default)]
    pub host: Option<String>,
    pub seq: u64,
    pub received_at: String,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_encoding: String,
    pub truncated: bool,
    #[serde(default)]
    pub status_sent: Option<u16>,
    #[serde(default)]
    pub remote_addr: Option<String>,
}

impl Envelope {
    /// Convenience accessor: headers as a map (last value wins per name).
    pub fn headers_map(&self) -> std::collections::HashMap<String, String> {
        self.headers.iter().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StaticResponse {
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BufferConfig {
    pub max_events: usize,
    pub max_age_s: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthConfig {
    pub bearer_env: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitConfig {
    pub burst: u32,
    pub refill_per_s: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForwardRule {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ChannelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mode: Option<String>, // ack | echo | static
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_response: Option<StaticResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer: Option<BufferConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward: Option<Vec<ForwardRule>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Overrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SigningSpec {
    pub profile: String,
}

/// Wire form of the replay source, matching docs/api.md §6 (`{"event_id": …}` etc.).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ReplaySourceWire {
    Event { event_id: String },
    Template { template: String },
    Inline { inline: Value },
}

impl Default for ReplayRequest {
    fn default() -> Self {
        Self {
            target_url: String::new(),
            timeout_ms: None,
            source: ReplaySourceWire::Event {
                event_id: String::new(),
            },
            overrides: None,
            vars: None,
            signing: None,
        }
    }
}

/// Request body for `POST /api/replay` (docs/api.md §6).
#[derive(Debug, Clone, Serialize)]
pub struct ReplayRequest {
    pub target_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    pub source: ReplaySourceWire,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Overrides>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vars: Option<serde_json::Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing: Option<SigningSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Delivery {
    pub status: Option<u16>,
    pub duration_ms: u64,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayResult {
    pub sent_event: Envelope,
    pub delivery: Delivery,
}

#[derive(Debug, Clone)]
pub struct SdkError {
    pub status: u16,
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for SdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.code, self.status, self.message)
    }
}
impl std::error::Error for SdkError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromSpec {
    Newest,
    Oldest,
    At(u64),
}

impl FromSpec {
    fn to_value(self) -> Value {
        match self {
            FromSpec::Newest => Value::String("newest".into()),
            FromSpec::Oldest => Value::String("oldest".into()),
            FromSpec::At(n) => Value::Number(n.into()),
        }
    }
}

/// Async client for one engine's control plane. Cheap to clone.
#[derive(Clone)]
pub struct RelayClient {
    http: reqwest::Client,
    base: String,
    capture_base: String,
    token: Option<String>,
}

impl RelayClient {
    /// `base` is the control-plane origin. The capture plane defaults to the same
    /// host with port+1 (8000 → 8001); pass an explicit origin when they differ.
    pub fn new(base: &str) -> reqwest::Result<Self> {
        Self::with_token(base, None)
    }

    pub fn with_token(base: &str, token: Option<String>) -> reqwest::Result<Self> {
        Self::with_capture_base(base, None, token)
    }

    pub fn with_capture_base(
        base: &str,
        capture_base: Option<&str>,
        token: Option<String>,
    ) -> reqwest::Result<Self> {
        let base = base.trim_end_matches('/').to_string();
        let derived = bump_port(&base);
        Ok(Self {
            http: reqwest::ClientBuilder::new()
                .pool_max_idle_per_host(16)
                .tcp_nodelay(true)
                .build()?,
            base,
            capture_base: capture_base
                .map(|s| s.trim_end_matches('/').to_string())
                .unwrap_or(derived),
            token,
        })
    }

    /// Provider-facing capture URL for the channel.
    pub fn capture_url(&self, channel: &str) -> String {
        format!("{}/c/{channel}", self.capture_base)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, SdkError> {
        let mut req = self.http.request(method.clone(), self.url(path));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        if let Some(b) = &body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| SdkError {
            status: 0,
            code: "transport".into(),
            message: e.to_string(),
        })?;
        let status = resp.status().as_u16();
        if status >= 300 && !(status == 201 && method == reqwest::Method::PUT) {
            let value: Value = resp.json().await.unwrap_or(Value::Null);
            return Err(SdkError {
                status,
                code: value["error"]["code"].as_str().unwrap_or("unknown").into(),
                message: value["error"]["message"].as_str().unwrap_or("").into(),
            });
        }
        resp.json().await.map_err(|e| SdkError {
            status,
            code: "bad_response".into(),
            message: e.to_string(),
        })
    }

    pub async fn upsert_channel(&self, name: &str, cfg: &ChannelConfig) -> Result<(), SdkError> {
        self.call(
            reqwest::Method::PUT,
            &format!("/api/channels/{name}"),
            Some(serde_json::to_value(cfg).map_err(|e| SdkError {
                status: 0,
                code: "serialize".into(),
                message: e.to_string(),
            })?),
        )
        .await
        .map(|_| ())
    }

    pub async fn delete_channel(&self, name: &str) -> Result<(), SdkError> {
        self.call(
            reqwest::Method::DELETE,
            &format!("/api/channels/{name}"),
            None,
        )
        .await
        .map(|_| ())
    }

    pub async fn history(
        &self,
        name: &str,
        cursor: Option<u64>,
        limit: usize,
    ) -> Result<Vec<Envelope>, SdkError> {
        let value = self
            .call(
                reqwest::Method::GET,
                &format!(
                    "/api/channels/{name}/events?cursor={}&limit={limit}",
                    cursor.unwrap_or(0)
                ),
                None,
            )
            .await?;
        parse_field(&value, "events")
    }

    pub async fn replay(&self, req: &ReplayRequest) -> Result<ReplayResult, SdkError> {
        let body = serde_json::to_value(req).map_err(|e| SdkError {
            status: 0,
            code: "serialize".into(),
            message: e.to_string(),
        })?;
        let value = self
            .call(reqwest::Method::POST, "/api/replay", Some(body))
            .await?;
        Ok(ReplayResult {
            sent_event: parse_field(&value, "sent_event")?,
            delivery: parse_field(&value, "delivery")?,
        })
    }

    /// Await exactly one live event. When `produce` is given it runs only after the
    /// engine confirms the subscription ("ok" frame) — the deterministic handshake-
    /// safe pattern for tests and demos.
    pub async fn once(
        &self,
        channel: &str,
        from: FromSpec,
        mut produce: Option<impl FnOnce()>,
    ) -> Result<Envelope, SdkError> {
        use tokio_tungstenite::tungstenite::Message;

        let ws_url = format!("ws{}/ws", &self.base[4..]);
        let mut ws = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| SdkError {
                status: 0,
                code: "websocket_error".into(),
                message: e.to_string(),
            })?
            .0;
        ws.send(Message::Text(
            serde_json::json!({
                "type": "subscribe",
                "id": "sdk-once",
                "channel": channel,
                "from": from.to_value()
            })
            .to_string()
            .into(),
        ))
        .await
        .map_err(|e| SdkError {
            status: 0,
            code: "websocket_error".into(),
            message: e.to_string(),
        })?;

        loop {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
                .await
                .map_err(|_| SdkError {
                    status: 0,
                    code: "websocket_timeout".into(),
                    message: "timed out".into(),
                })?
                .ok_or_else(|| SdkError {
                    status: 0,
                    code: "websocket_closed".into(),
                    message: "socket closed".into(),
                })?
                .map_err(|e| SdkError {
                    status: 0,
                    code: "websocket_error".into(),
                    message: e.to_string(),
                })?;

            if let Message::Text(text) = msg {
                let frame: Value = serde_json::from_str(&text).map_err(|e| SdkError {
                    status: 0,
                    code: "bad_frame".into(),
                    message: e.to_string(),
                })?;
                match frame["type"].as_str() {
                    Some("ok") => {
                        if let Some(produce) = produce.take() {
                            produce();
                        }
                    }
                    Some("event") => {
                        let env: Envelope = serde_json::from_value(frame["event"].clone())
                            .map_err(|e| SdkError {
                                status: 0,
                                code: "bad_frame".into(),
                                message: e.to_string(),
                            })?;
                        return Ok(env);
                    }
                    Some("error") => {
                        return Err(SdkError {
                            status: 400,
                            code: frame["error"]["code"].as_str().unwrap_or("unknown").into(),
                            message: frame["error"]["message"].as_str().unwrap_or("").into(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}

fn parse_field<T: serde::de::DeserializeOwned>(value: &Value, key: &str) -> Result<T, SdkError> {
    serde_json::from_value(value[key].clone()).map_err(|e| SdkError {
        status: 200,
        code: "bad_response".into(),
        message: e.to_string(),
    })
}

fn bump_port(origin: &str) -> String {
    match origin.rfind(':') {
        Some(idx) => match origin[idx + 1..].parse::<u16>() {
            Ok(port) => format!("{}:{}", &origin[..idx], port + 1),
            Err(_) => origin.to_string(),
        },
        None => origin.to_string(),
    }
}
