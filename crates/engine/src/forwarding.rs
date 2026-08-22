//! Forwarding rules (M3): on capture, auto-deliver transformed copies of the event
//! as fresh webhook requests to configured targets.

use crate::store::Channel;
use crate::AppState;
use base64::Engine as _;
use std::sync::Arc;

pub async fn apply(state: &AppState, channel: &Arc<Channel>, env: &Arc<crate::envelope::Envelope>) {
    let rules = channel.config().forward.clone();
    for rule in rules {
        let state = state.clone();
        let channel_name = channel.name.clone();
        let env = env.clone();
        let profile = rule.profile.clone();
        let extra_headers = rule.headers.clone();
        tokio::spawn(async move {
            // Two attempts total: transient connect blips get one backoff retry.
            // Definitive responses never retry. The whole delivery is bounded so a
            // stalled target cannot leak the task forever.
            const FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
            for attempt in 1..=2u32 {
                let outcome = match tokio::time::timeout(
                    FORWARD_TIMEOUT,
                    deliver(
                        &state,
                        &channel_name,
                        &env,
                        &rule.url,
                        profile.as_ref(),
                        &extra_headers,
                    ),
                )
                .await
                {
                    Err(_) => Err(ForwardError::Transient(anyhow::anyhow!(
                        "delivery timed out"
                    ))),
                    Ok(r) => r,
                };
                match outcome {
                    Ok(()) => break,
                    Err(ForwardError::Transient(e)) if attempt < 2 => {
                        tracing::debug!(attempt, error = %e, url = %rule.url, "forward retrying");
                        tokio::time::sleep(std::time::Duration::from_millis(200 * attempt as u64))
                            .await;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, channel = %channel_name, url = %rule.url, "forward failed");
                        break;
                    }
                }
            }
        });
    }
}

enum ForwardError {
    Transient(anyhow::Error),
    Permanent(anyhow::Error),
}

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForwardError::Transient(e) | ForwardError::Permanent(e) => write!(f, "{e}"),
        }
    }
}

async fn deliver(
    state: &AppState,
    _channel: &str,
    env: &Arc<crate::envelope::Envelope>,
    url: &str,
    profile: Option<&String>,
    extra_headers: &[(String, String)],
) -> Result<(), ForwardError> {
    // Destination path: rule URL's path + the captured suffix (append semantics, §6).
    let mut dest =
        url::Url::parse(url).map_err(|e| ForwardError::Permanent(anyhow::Error::new(e)))?;
    let base = dest.path().trim_end_matches('/').to_string();
    let suffix = if env.path.is_empty() || env.path == "/" {
        String::new()
    } else {
        env.path.clone()
    };
    dest.set_path(&format!("{base}{suffix}"));
    dest.set_query(env.query.as_deref());

    let body = match env.body_encoding {
        crate::envelope::BodyEncoding::Utf8 => bytes::Bytes::from(env.body.clone()),
        crate::envelope::BodyEncoding::Base64 => bytes::Bytes::from(
            base64::engine::general_purpose::STANDARD
                .decode(&env.body)
                .map_err(|e| ForwardError::Permanent(anyhow::Error::new(e)))?,
        ),
    };

    let method = reqwest::Method::from_bytes(env.method.as_bytes())
        .map_err(|e| ForwardError::Permanent(anyhow::Error::new(e)))?;
    let mut request = state.http.request(method, dest.as_str());
    for (k, v) in env.headers.iter().filter(|(k, _)| !is_hop_by_hop(k)) {
        request = request.header(k, v);
    }
    for (k, v) in extra_headers {
        request = request.header(k, v);
    }
    if let Some(profile) = profile {
        let (scheme, secret) = state
            .profiles
            .resolve(profile)
            .map_err(ForwardError::Permanent)?;
        let now = crate::envelope::now_utc().unix_timestamp();
        for (k, v) in crate::signing::sign_for(scheme, &secret, &body, now, Some(dest.as_str())) {
            request = request.header(k, v);
        }
    }
    let resp = request.body(body).send().await.map_err(|e| {
        if e.is_connect() || e.is_timeout() {
            ForwardError::Transient(anyhow::Error::new(e))
        } else {
            ForwardError::Permanent(anyhow::Error::new(e))
        }
    })?;
    tracing::debug!(status = %resp.status(), url = %dest, "forwarded");
    Ok(())
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name,
        "host"
            | "content-length"
            | "connection"
            | "transfer-encoding"
            | "accept-encoding"
            | "keep-alive"
    )
}
