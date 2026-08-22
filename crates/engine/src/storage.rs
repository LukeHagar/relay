//! Optional durable layer (M2): SQLite persistence for events and channel configs.
//! In-memory buffers stay authoritative for live subscriptions; the DB makes
//! history and channels survive restarts and keeps per-channel `seq` monotonic.

use crate::envelope::{BodyEncoding, Envelope};
use crate::store::ChannelConfig;
use base64::Engine as _;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

pub struct Db {
    /// `pub(crate)` so the billing module (same crate) can extend persistence.
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    channel TEXT NOT NULL,
    seq INTEGER NOT NULL,
    received_at TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    query TEXT,
    host TEXT,
    headers TEXT NOT NULL,
    body BLOB NOT NULL,
    body_encoding TEXT NOT NULL,
    truncated INTEGER NOT NULL,
    status_sent INTEGER,
    remote_addr TEXT
);
CREATE INDEX IF NOT EXISTS idx_events_channel_seq ON events(channel, seq);
CREATE TABLE IF NOT EXISTS channels (
    name TEXT PRIMARY KEY,
    config TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    status TEXT NOT NULL,
    stripe_customer_id TEXT,
    stripe_subscription_id TEXT,
    current_period_end_unix INTEGER
);
"#;

pub struct LoadedChannel {
    pub name: String,
    pub config: ChannelConfig,
    pub created_at: String,
    /// Highest persisted `seq` for the channel (0 when empty); the channel resumes
    /// allocating above this so cursors survive restarts.
    pub last_allocated_seq: u64,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Arc<Self>> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Arc::new(Self {
            conn: Mutex::new(conn),
        }))
    }

    pub fn insert_event(&self, env: &Envelope) -> anyhow::Result<()> {
        let headers = serde_json::to_string(&env.headers)?;
        let raw = match env.body_encoding {
            BodyEncoding::Utf8 => env.body.clone().into_bytes(),
            BodyEncoding::Base64 => base64::engine::general_purpose::STANDARD.decode(&env.body)?,
        };
        self.conn.lock().execute(
            "INSERT OR IGNORE INTO events
             (id, channel, seq, received_at, method, path, query, host, headers, body, body_encoding, truncated, status_sent, remote_addr)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            rusqlite::params![
                env.id,
                env.channel,
                env.seq as i64,
                env.received_at,
                env.method,
                env.path,
                env.query,
                env.host,
                headers,
                raw,
                env.body_encoding_str(),
                env.truncated as i64,
                env.status_sent.map(|s| s as i64),
                env.remote_addr,
            ],
        )?;
        Ok(())
    }

    pub fn events(
        &self,
        channel: &str,
        cursor: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<Vec<Envelope>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, channel, seq, received_at, method, path, query, host, headers, body, body_encoding, truncated, status_sent, remote_addr
             FROM events WHERE channel = ?1 AND (?2 IS NULL OR seq > ?2)
             ORDER BY seq ASC LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![channel, cursor.map(|c| c as i64), limit as i64],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Vec<u8>>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, Option<i64>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                ))
            },
        )?;
        let mut out = Vec::new();
        for r in rows {
            let (
                id,
                channel,
                seq,
                received_at,
                method,
                path,
                query,
                host,
                headers_json,
                raw,
                encoding,
                truncated,
                status_sent,
                remote_addr,
            ) = r?;
            let (body, body_encoding) = match encoding.as_str() {
                "base64" => (
                    base64::engine::general_purpose::STANDARD.encode(&raw),
                    BodyEncoding::Base64,
                ),
                _ => (
                    String::from_utf8_lossy(&raw).into_owned(),
                    BodyEncoding::Utf8,
                ),
            };
            out.push(Envelope {
                v: 1,
                id,
                channel,
                host,
                seq: seq as u64,
                received_at,
                method,
                path,
                query,
                headers: serde_json::from_str(&headers_json)?,
                body,
                body_encoding,
                truncated: truncated != 0,
                status_sent: status_sent.map(|s| s as u16),
                remote_addr,
            });
        }
        Ok(out)
    }

    pub fn insert_channel(
        &self,
        name: &str,
        config: &ChannelConfig,
        created_at: &str,
    ) -> anyhow::Result<()> {
        self.conn.lock().execute(
            "INSERT INTO channels (name, config, created_at) VALUES (?1,?2,?3)
             ON CONFLICT(name) DO UPDATE SET config = excluded.config",
            rusqlite::params![name, serde_json::to_string(config)?, created_at],
        )?;
        Ok(())
    }

    pub fn delete_channel(&self, name: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM channels WHERE name = ?1", [name])?;
        conn.execute("DELETE FROM events WHERE channel = ?1", [name])?;
        Ok(())
    }

    pub fn load_channels(&self) -> anyhow::Result<Vec<LoadedChannel>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT name, config, created_at FROM channels")?;
        let mut out = Vec::new();
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for r in rows {
            let (name, config_json, created_at) = r?;
            let config: ChannelConfig = serde_json::from_str(&config_json)?;
            let next_seq = conn.query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM events WHERE channel = ?1",
                [&name],
                |row| row.get::<_, i64>(0),
            )? as u64;
            out.push(LoadedChannel {
                name,
                config,
                created_at,
                last_allocated_seq: next_seq,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{build_envelope, CaptureInput};

    fn env(seq: u64, body: &[u8]) -> Envelope {
        build_envelope(
            &CaptureInput {
                channel: "t".into(),
                host: Some("h".into()),
                method: "POST".into(),
                raw_path: "/x%20y".into(),
                raw_query: Some("b=2&a=1".into()),
                headers: vec![
                    ("content-type".into(), "application/json".into()),
                    ("x-dup".into(), "1".into()),
                    ("x-dup".into(), "2".into()),
                ],
                body: bytes::Bytes::copy_from_slice(body),
                truncated: false,
                status_sent: 200,
                remote_addr: None,
            },
            seq,
            crate::envelope::now_utc(),
        )
    }

    #[test]
    fn event_roundtrip_preserves_fidelity() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        // binary body → base64 path
        let binary = [0u8, 159, 146, 150];
        db.insert_event(&env(1, b"{\"a\":1}")).unwrap();
        db.insert_event(&env(2, &binary)).unwrap();

        let loaded = db.events("t", None, 100).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].body, "{\"a\":1}");
        assert_eq!(loaded[0].body_encoding, BodyEncoding::Utf8);
        assert_eq!(loaded[0].path, "/x%20y"); // encoding untouched
        assert_eq!(loaded[0].query.as_deref(), Some("b=2&a=1"));
        assert_eq!(loaded[0].headers.len(), 3); // duplicates kept
        assert_eq!(loaded[1].body_encoding, BodyEncoding::Base64);
        assert_eq!(loaded[1].id.len(), 26); // ULID

        // cursor pagination
        assert_eq!(db.events("t", Some(1), 100).unwrap()[0].seq, 2);
    }

    #[test]
    fn channels_roundtrip_with_seq_continuity() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        let cfg = ChannelConfig {
            response_mode: Some(crate::store::ResponseMode::Echo),
            ..Default::default()
        };
        db.insert_channel("gh", &cfg, "2026-08-21T00:00:00.000Z")
            .unwrap();
        let mut e = env(7, b"x");
        e.channel = "gh".into();
        db.insert_event(&e).unwrap();

        let loaded = db.load_channels().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "gh");
        assert_eq!(
            loaded[0].config.response_mode(),
            crate::store::ResponseMode::Echo
        );
        assert_eq!(loaded[0].last_allocated_seq, 7); // resumes above persisted max
    }
}
