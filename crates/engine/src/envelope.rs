//! Event envelope (§4 of docs/api.md) — one captured request ⇔ one immutable event.

use base64::Engine as _;
use serde::Serialize;
use time::format_description::BorrowedFormatItem;
use time::OffsetDateTime;

/// RFC 3339 UTC with millisecond precision and `Z` suffix, per the contract.
const RFC3339_MS: &[BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
);

pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn format_rfc3339_ms(t: OffsetDateTime) -> String {
    t.format(RFC3339_MS)
        .expect("rfc3339 formatting cannot fail")
}

pub fn truncate_ms(t: OffsetDateTime) -> OffsetDateTime {
    let ms = t.nanosecond() / 1_000_000;
    t.replace_nanosecond(ms * 1_000_000)
        .expect("ms truncation is in range")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyEncoding {
    Utf8,
    Base64,
}

impl Envelope {
    pub fn body_encoding_str(&self) -> &'static str {
        match self.body_encoding {
            BodyEncoding::Utf8 => "utf8",
            BodyEncoding::Base64 => "base64",
        }
    }
}

/// The wire contract. Field order and casing are contractual (§4).
#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    pub v: u8,
    pub id: String,
    pub channel: String,
    pub host: Option<String>,
    pub seq: u64,
    pub received_at: String,
    pub method: String,
    /// Faithful path: full request path in hosted mode; channel-root suffix locally.
    pub path: String,
    /// Raw query string, original order and encoding; null when absent.
    pub query: Option<String>,
    /// `[name, value]` pairs in arrival order, duplicates preserved, names lowercased.
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_encoding: BodyEncoding,
    pub truncated: bool,
    pub status_sent: Option<u16>,
    pub remote_addr: Option<String>,
}

/// Everything needed to mint an envelope; the channel assigns `seq` under its lock.
#[derive(Clone)]
pub struct CaptureInput {
    pub channel: String,
    pub host: Option<String>,
    pub method: String,
    /// Raw (still percent-encoded) request-target path as received on the wire.
    pub raw_path: String,
    pub raw_query: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: bytes::Bytes,
    pub truncated: bool,
    pub status_sent: u16,
    pub remote_addr: Option<String>,
}

pub fn encode_body(body: &[u8]) -> (String, BodyEncoding) {
    match std::str::from_utf8(body) {
        Ok(s) => (s.to_owned(), BodyEncoding::Utf8),
        Err(_) => (
            base64::engine::general_purpose::STANDARD.encode(body),
            BodyEncoding::Base64,
        ),
    }
}

/// Everything except `seq` — computed outside the channel lock so the critical
/// section only pays for the seq bump and the ring-buffer insert.
pub fn prepare_envelope(input: CaptureInput, now: OffsetDateTime) -> Envelope {
    let (body, body_encoding) = encode_body(&input.body);
    Envelope {
        v: 1,
        id: ulid::Ulid::new().to_string(),
        channel: input.channel,
        host: input.host,
        seq: 0,
        received_at: format_rfc3339_ms(truncate_ms(now)),
        method: input.method.to_ascii_uppercase(),
        path: input.raw_path,
        query: input.raw_query,
        headers: input.headers,
        body,
        body_encoding,
        truncated: input.truncated,
        status_sent: Some(input.status_sent),
        remote_addr: input.remote_addr,
    }
}

/// Backward-compatible wrapper (tests / external callers holding a reference).
pub fn build_envelope(input: &CaptureInput, seq: u64, now: OffsetDateTime) -> Envelope {
    let mut env = prepare_envelope(input.clone(), now);
    env.seq = seq;
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn utf8_bodies_stay_utf8() {
        let (s, enc) = encode_body(b"{\"a\":1}");
        assert_eq!(enc, BodyEncoding::Utf8);
        assert_eq!(s, "{\"a\":1}");
    }

    #[test]
    fn binary_bodies_base64_round_trip() {
        let bytes = [0u8, 159, 146, 150, 255];
        let (s, enc) = encode_body(&bytes);
        assert_eq!(enc, BodyEncoding::Base64);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&s)
            .unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn timestamps_have_millisecond_precision_and_z() {
        let t = datetime!(2026-08-21 12:00:00.123456789 UTC);
        assert_eq!(
            format_rfc3339_ms(truncate_ms(t)),
            "2026-08-21T12:00:00.123Z"
        );
        let whole = datetime!(2026-08-21 12:00:00 UTC);
        assert_eq!(
            format_rfc3339_ms(truncate_ms(whole)),
            "2026-08-21T12:00:00.000Z"
        );
    }

    #[test]
    fn envelope_carries_fidelity_fields() {
        let input = CaptureInput {
            channel: "gh".into(),
            host: Some("gh.example.zone".into()),
            method: "POST".into(),
            raw_path: "/a%2Fb/../c".into(),
            raw_query: Some("b=2&a=1".into()),
            headers: vec![
                ("content-type".into(), "application/json".into()),
                ("x-dup".into(), "1".into()),
                ("x-dup".into(), "2".into()),
            ],
            body: bytes::Bytes::from_static(b"hi"),
            truncated: false,
            status_sent: 200,
            remote_addr: Some("10.0.0.1:5".to_string()),
        };
        let env = build_envelope(&input, 7, datetime!(2026-08-21 12:00:00 UTC));
        assert_eq!(env.v, 1);
        assert_eq!(env.seq, 7);
        assert_eq!(env.path, "/a%2Fb/../c"); // untouched
        assert_eq!(env.query.as_deref(), Some("b=2&a=1")); // original order
        assert_eq!(env.headers.len(), 3); // duplicates kept
        assert_eq!(env.status_sent, Some(200));
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["v"], 1);
        assert_eq!(json["body_encoding"], "utf8");
    }
}
