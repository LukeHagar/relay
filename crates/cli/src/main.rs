//! `relay` — CLI for the Relay engine. The CLI is a client over the wire contract
//! (`docs/api.md`): everything it does is possible with plain HTTP/WS.

/// mimalloc: multithreaded request handling hammers malloc/free across 32 tokio
/// workers; the system allocator's contention showed up as tail latency.
#[global_allocator]
static GLOBAL_ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio_tungstenite::tungstenite::Message;

#[derive(Parser)]
#[command(
    name = "relay",
    version,
    about = "Postman for webhooks — capture, inspect, replay"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the engine (control + capture planes).
    Serve {
        /// Control plane address (`/api/*`, `/ws`, SSE)
        #[arg(long, env = "RELAY_CONTROL_ADDR", default_value = "127.0.0.1:8000")]
        control_addr: SocketAddr,
        /// Capture plane address (webhook ingestion)
        #[arg(long, env = "RELAY_CAPTURE_ADDR", default_value = "127.0.0.1:8001")]
        capture_addr: SocketAddr,
        /// Require this bearer token on the control plane
        #[arg(long, env = "RELAY_TOKEN")]
        token: Option<String>,
        /// Hosted mode: accept `{slug}.{zone}` hosts on the capture plane
        #[arg(long, env = "RELAY_CAPTURE_ZONE")]
        capture_zone: Option<String>,
        /// Per-request capture body limit in bytes
        #[arg(long, env = "RELAY_MAX_BODY_BYTES", default_value_t = 1024 * 1024)]
        max_body_bytes: usize,
        /// Engine config file (signing profiles)
        #[arg(long, env = "RELAY_CONFIG")]
        config: Option<PathBuf>,
        /// SQLite database — history and channels survive restarts
        #[arg(long, env = "RELAY_DB")]
        db: Option<PathBuf>,
        /// Enable the billing layer (Stripe plans, quotas, checkout, webhooks)
        #[arg(long, env = "RELAY_BILLING_ENABLED")]
        billing_enabled: bool,
        /// Stripe Price ID for the Pro plan (checkout sessions)
        #[arg(long, env = "RELAY_STRIPE_PRICE_PRO")]
        stripe_price_pro: Option<String>,
        /// Stripe webhook signing secret (else STRIPE_WEBHOOK_SECRET env)
        #[arg(long, env = "RELAY_STRIPE_WEBHOOK_SECRET")]
        stripe_webhook_secret: Option<String>,
        /// Hard channel cap without billing
        #[arg(long, env = "RELAY_MAX_CHANNELS")]
        max_channels: Option<usize>,
        /// Permit inline `secret` in replay requests
        #[arg(long, default_value_t = false)]
        allow_inline_secret: bool,
    },
    /// Subscribe to a channel and print events as they arrive.
    Listen {
        /// Channel name
        channel: String,
        /// Exit after receiving N events
        #[arg(long)]
        max_events: Option<usize>,
        /// Backlog start: `newest`, `oldest`, or a sequence number
        #[arg(long, default_value = "newest")]
        from: String,
        /// Control-plane base URL
        #[arg(long, env = "RELAY_URL", default_value = "ws://127.0.0.1:8000/ws")]
        url: String,
        /// Control-plane bearer token
        #[arg(long, env = "RELAY_TOKEN")]
        token: Option<String>,
    },
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Check the local environment: engine reachability, auth, durability.
    Doctor {
        /// Control-plane base URL
        #[arg(long, env = "RELAY_HTTP_URL", default_value = "http://127.0.0.1:8000")]
        url: String,
        /// Capture-plane base URL
        #[arg(
            long,
            env = "RELAY_CAPTURE_HTTP_URL",
            default_value = "http://127.0.0.1:8001"
        )]
        capture_url: String,
        /// Control-plane bearer token
        #[arg(long, env = "RELAY_TOKEN")]
        token: Option<String>,
    },
    /// Interactive WebSocket client: send lines as text frames, print inbound frames.
    Ws {
        /// WebSocket URL to connect to
        url: String,
        /// Send one message, print responses for 2s, then exit (omit for interactive)
        #[arg(long)]
        message: Option<String>,
        /// Bearer token appended as ?token= (browser-compatible form)
        #[arg(long, env = "RELAY_TOKEN")]
        token: Option<String>,
    },
    /// Export/import template fixture packs (.relaypack.json).
    Templates {
        #[command(subcommand)]
        cmd: TemplatesCmd,
    },
    /// Send a one-off request (inline replay without a stored source).
    Send {
        /// Full target URL (path is used verbatim, not appended)
        #[arg(long)]
        target: String,
        #[arg(long, default_value = "POST")]
        method: String,
        /// Header, `Name=value`; repeatable
        #[arg(long = "header")]
        headers: Vec<String>,
        /// Request body (string)
        #[arg(long)]
        data: Option<String>,
        /// Signing profile name
        #[arg(long)]
        profile: Option<String>,
        #[arg(long, default_value_t = 10_000)]
        timeout_ms: u64,
        /// Control-plane base URL
        #[arg(long, env = "RELAY_HTTP_URL", default_value = "http://127.0.0.1:8000")]
        url: String,
        /// Control-plane bearer token
        #[arg(long, env = "RELAY_TOKEN")]
        token: Option<String>,
    },
    /// Replay a stored/template/inline event to a target URL.
    Replay {
        /// Target base URL (origin); overrides apply on top of the source
        #[arg(long)]
        target: String,
        /// Stored event id (ULID)
        #[arg(long, group = "source")]
        event: Option<String>,
        /// Template ref like `stripe/invoice.paid@1`
        #[arg(long, group = "source")]
        template: Option<String>,
        /// Inline request JSON: {"method":"POST","path":"/","headers":[],"body":"…"}
        #[arg(long, group = "source")]
        inline: Option<String>,
        /// Template variable, `key=value`; repeatable
        #[arg(long = "var")]
        vars: Vec<String>,
        /// Override header, `Name=value`; repeatable
        #[arg(long = "header")]
        headers: Vec<String>,
        /// Override path
        #[arg(long)]
        path: Option<String>,
        /// Signing profile name (secrets resolve from engine config)
        #[arg(long)]
        profile: Option<String>,
        /// Inline signing secret (requires `--allow-inline-secret` on the engine)
        #[arg(long)]
        secret: Option<String>,
        /// Signing scheme when using an inline secret
        #[arg(long)]
        scheme: Option<String>,
        /// Delivery timeout in milliseconds
        #[arg(long, default_value_t = 10_000)]
        timeout_ms: u64,
        /// Send the same replay N times (burst/load mode)
        #[arg(long, default_value_t = 1)]
        repeat: u32,
        /// Pause between repeats in milliseconds
        #[arg(long, default_value_t = 0)]
        interval_ms: u64,
        /// Fail (exit 1) if p95 latency exceeds this many ms
        #[arg(long)]
        assert_p95_ms: Option<u64>,
        /// Control-plane base URL
        #[arg(long, env = "RELAY_HTTP_URL", default_value = "http://127.0.0.1:8000")]
        url: String,
        /// Control-plane bearer token
        #[arg(long, env = "RELAY_TOKEN")]
        token: Option<String>,
    },
}

fn parse_kv(pairs: &[String], what: &str) -> anyhow::Result<Vec<(String, String)>> {
    pairs
        .iter()
        .map(|kv| {
            kv.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| anyhow::anyhow!("invalid {what} '{kv}', expected key=value"))
        })
        .collect()
}

async fn api_post(
    url: &str,
    path: &str,
    token: &Option<String>,
    body: serde_json::Value,
) -> anyhow::Result<reqwest::Response> {
    let mut req = reqwest::Client::new()
        .post(format!("{url}{path}"))
        .json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    Ok(req.send().await?)
}

impl Command {
    async fn run(self) -> anyhow::Result<()> {
        match self {
            Command::Serve {
                control_addr,
                capture_addr,
                token,
                capture_zone,
                max_body_bytes,
                config,
                db,
                billing_enabled,
                stripe_price_pro,
                stripe_webhook_secret,
                max_channels,
                allow_inline_secret,
            } => {
                let cfg = relay_engine::config::Config {
                    control_addr,
                    capture_addr,
                    token,
                    capture_zone,
                    max_body_bytes,
                    config_path: config,
                    db_path: db,
                    max_channels,
                    billing_enabled,
                    stripe_price_pro,
                    stripe_webhook_secret,
                    allow_inline_secret,
                };
                relay_engine::serve(cfg).await
            }
            Command::Listen {
                channel,
                from,
                url,
                max_events,
                token,
            } => {
                let from_value: serde_json::Value = match from.as_str() {
                    "newest" | "oldest" => serde_json::Value::String(from.clone()),
                    other => other
                        .parse::<u64>()
                        .map_err(|_| anyhow::anyhow!("--from must be newest, oldest, or a number"))?
                        .into(),
                };
                // ?token= is the documented browser fallback; header form preferred.
                let connect_url = match (&token, url.contains('?')) {
                    (Some(_), _) if !url.contains("token=") => {
                        format!("{url}?token={}", token.clone().unwrap_or_default())
                    }
                    _ => url.clone(),
                };
                let (mut ws, _) = tokio_tungstenite::connect_async(&connect_url).await?;
                ws.send(Message::Text(
                    serde_json::json!({
                        "type": "subscribe",
                        "id": format!("cli-{}", std::process::id()),
                        "channel": channel,
                        "from": from_value,
                    })
                    .to_string()
                    .into(),
                ))
                .await?;

                let mut received = 0usize;
                while let Some(msg) = ws.next().await {
                    match msg? {
                        Message::Text(t) => {
                            let v: serde_json::Value = serde_json::from_str(&t)?;
                            match v["type"].as_str() {
                                Some("event") => {
                                    println!("{}", serde_json::to_string_pretty(&v["event"])?);
                                    received += 1;
                                    if max_events.is_some_and(|n| received >= n) {
                                        eprintln!("--max-events reached ({received}); done");
                                        return Ok(());
                                    }
                                }
                                Some("error") => anyhow::bail!("engine error: {}", v["error"]),
                                Some("channel_closed") => {
                                    eprintln!("channel closed");
                                    break;
                                }
                                Some("ok") => eprintln!("subscribed (last_seq {})", v["last_seq"]),
                                _ => eprintln!("{t}"),
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
                Ok(())
            }
            Command::Ws {
                url,
                message,
                token,
            } => ws_client(url, message, token).await,
            Command::Templates { cmd } => match cmd {
                TemplatesCmd::Export { refs, dir, out } => templates_export(refs, dir, out).await,
                TemplatesCmd::Import { pack, into } => templates_import(pack, into).await,
            },
            Command::Send {
                target,
                method,
                headers,
                data,
                profile,
                timeout_ms,
                url,
                token,
            } => {
                let parsed_headers = parse_kv(&headers, "header")?;
                let parsed_target =
                    url::Url::parse(&target).map_err(|e| anyhow::anyhow!("invalid target: {e}"))?;
                let path = parsed_target.path().to_string();
                let query = parsed_target.query().map(|q| q.to_string());
                let mut inline = serde_json::json!({
                    "method": method,
                    "path": path,
                    "headers": parsed_headers,
                    "body": data.unwrap_or_default(),
                });
                if let Some(q) = &query {
                    inline["query"] = serde_json::Value::String(q.clone());
                }
                // `send` targets a full endpoint: replace the path rather than
                // appending the source path onto it.
                let mut overrides = serde_json::json!({ "path": path });
                if let Some(q) = &query {
                    overrides["query"] = serde_json::Value::String(q.clone());
                }
                let mut body = serde_json::json!({
                    "target_url": target,
                    "timeout_ms": timeout_ms,
                    "source": { "inline": inline },
                    "overrides": overrides,
                });
                if let Some(p) = profile {
                    body["signing"] = serde_json::json!({ "profile": p });
                }
                let resp = api_post(&url, "/api/replay", &token, body).await?;
                let status = resp.status();
                let payload: serde_json::Value = resp.json().await?;
                if status.is_success() {
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                    Ok(())
                } else {
                    anyhow::bail!("send failed ({}): {}", status, payload["error"])
                }
            }
            Command::Completions { shell } => {
                generate(shell, &mut Cli::command(), "relay", &mut std::io::stdout());
                eprintln!(
                    "# install: relay completions {shell} > ~/relay-completions.{shell} then source it in your shell rc"
                );
                Ok(())
            }
            Command::Doctor {
                url,
                capture_url,
                token,
            } => doctor(&url, &capture_url, &token).await,
            Command::Replay {
                target,
                event,
                template,
                inline,
                vars,
                headers,
                path,
                profile,
                secret,
                scheme,
                timeout_ms,
                repeat,
                interval_ms,
                assert_p95_ms,
                url,
                token,
            } => {
                let source = match (event, template, inline) {
                    (Some(e), _, _) => serde_json::json!({ "event_id": e }),
                    (_, Some(t), _) => serde_json::json!({ "template": t }),
                    (_, _, Some(i)) => {
                        let parsed: serde_json::Value = serde_json::from_str(&i)
                            .map_err(|e| anyhow::anyhow!("invalid --inline JSON: {e}"))?;
                        serde_json::json!({ "inline": parsed })
                    }
                    _ => anyhow::bail!("one of --event, --template, or --inline is required"),
                };
                let mut body = serde_json::json!({
                    "target_url": target,
                    "timeout_ms": timeout_ms,
                    "source": source,
                });
                if !vars.is_empty() {
                    let map: serde_json::Map<String, serde_json::Value> = vars
                        .iter()
                        .map(|kv| {
                            kv.split_once('=')
                                .map(|(k, v)| {
                                    (k.to_string(), serde_json::Value::String(v.to_string()))
                                })
                                .unwrap()
                        })
                        .collect();
                    body["vars"] = serde_json::Value::Object(map);
                }
                if !headers.is_empty() || path.is_some() {
                    let mut o = serde_json::Map::new();
                    if let Some(p) = path {
                        o.insert("path".into(), p.into());
                    }
                    if !headers.is_empty() {
                        o.insert(
                            "headers".into(),
                            serde_json::to_value(parse_kv(&headers, "header")?)?,
                        );
                    }
                    body["overrides"] = serde_json::Value::Object(o);
                }
                if profile.is_some() || secret.is_some() || scheme.is_some() {
                    body["signing"] = serde_json::json!({
                        "profile": profile,
                        "scheme": scheme,
                        "secret": secret,
                    });
                }

                let mut durations: Vec<u64> = Vec::new();
                let mut failures = 0;
                for i in 0..repeat {
                    if i > 0 && interval_ms > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
                    }
                    let resp = api_post(&url, "/api/replay", &token, body.clone()).await?;
                    let status = resp.status();
                    let payload: serde_json::Value = resp.json().await?;
                    if !status.is_success() {
                        failures += 1;
                        let code = payload["error"]["code"].as_str().unwrap_or("");
                        let hint = hint_for(code).unwrap_or("");
                        eprintln!(
                            "[{i}] replay failed ({}): {}{}",
                            status,
                            payload["error"],
                            if hint.is_empty() {
                                String::new()
                            } else {
                                format!(" — {hint}")
                            }
                        );
                        continue;
                    }
                    if repeat == 1 {
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                    }
                    let d = &payload["delivery"];
                    if d["error"].is_string() {
                        failures += 1;
                        eprintln!("[{i}] delivery error: {}", d["error"]);
                    } else {
                        durations.push(d["duration_ms"].as_u64().unwrap_or(0));
                    }
                }
                if repeat > 1 {
                    durations.sort_unstable();
                    let p = |q: f64| -> u64 {
                        let idx = ((q * (durations.len() as f64 - 1.0)).round()) as usize;
                        durations.get(idx).copied().unwrap_or(0)
                    };
                    eprintln!(
                        "sent={} failed={} min={}ms p50={}ms p95={}ms max={}ms",
                        durations.len() + failures,
                        failures,
                        durations.first().copied().unwrap_or(0),
                        p(0.5),
                        p(0.95),
                        durations.last().copied().unwrap_or(0),
                    );
                }
                if let Some(cap) = assert_p95_ms {
                    let mut sorted = durations.clone();
                    sorted.sort_unstable();
                    let idx = ((0.95 * (sorted.len() as f64 - 1.0)).round()) as usize;
                    let p95 = sorted.get(idx).copied().unwrap_or(u64::MAX);
                    if p95 > cap {
                        anyhow::bail!("latency assertion failed: p95 {p95}ms > {cap}ms");
                    }
                    eprintln!("p95 assertion ok: {p95}ms <= {cap}ms");
                }
                if failures > 0 {
                    anyhow::bail!("{failures} replay(s) failed");
                }
                Ok(())
            }
        }
    }
}

#[derive(Subcommand)]
enum TemplatesCmd {
    /// Write a shareable .relaypack.json from bundled + on-disk templates
    Export {
        /// Template refs (provider/name or provider/name@v); omit for all
        #[arg(long = "ref")]
        refs: Vec<String>,
        /// Extra directory to read user templates from
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Write pack templates into a templates/ directory (bundled + pack merged)
    Import {
        /// .relaypack.json produced by `templates export`
        pack: PathBuf,
        /// Destination directory (defaults to ./templates)
        #[arg(long, default_value = "templates")]
        into: PathBuf,
    },
}

async fn templates_export(
    refs: Vec<String>,
    dir: Option<PathBuf>,
    out: PathBuf,
) -> anyhow::Result<()> {
    let registry =
        relay_engine::templates::Registry::load(relay_engine::BUNDLED_TEMPLATES, dir.as_deref())?;
    let all = registry.list();
    let chosen: Vec<_> = if refs.is_empty() {
        all
    } else {
        refs.iter()
            .map(|r| {
                registry
                    .get(r)
                    .ok_or_else(|| anyhow::anyhow!("template '{r}' not found"))
            })
            .collect::<Result<_, _>>()?
    };
    let pack = serde_json::json!({
        "manifest": {
            "kind": "relaypack",
            "version": 1,
            "exported_at": relay_engine::envelope::format_rfc3339_ms(
                relay_engine::envelope::truncate_ms(relay_engine::envelope::now_utc())
            ),
            "count": chosen.len(),
        },
        "templates": chosen,
    });
    let text = serde_json::to_string_pretty(&pack)?;
    std::fs::write(&out, text)?;
    eprintln!("wrote {} templates to {}", chosen.len(), out.display());
    Ok(())
}

async fn templates_import(pack: PathBuf, into: PathBuf) -> anyhow::Result<()> {
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&pack)?)?;
    if value["manifest"]["kind"].as_str() != Some("relaypack") {
        anyhow::bail!("not a relaypack file (manifest.kind != relaypack)");
    }
    let templates = value["templates"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("pack has no templates array"))?;
    std::fs::create_dir_all(&into)?;
    let mut written = 0;
    for t in templates {
        let provider = t["provider"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("template missing provider"))?;
        let name = t["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("template missing name"))?;
        let v = t["v"].as_u64().unwrap_or(1);
        let dir = into.join(provider);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}@{v}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(t)?)?;
        eprintln!("wrote {}", path.display());
        written += 1;
    }
    eprintln!("imported {written} templates into {}", into.display());
    Ok(())
}

async fn ws_client(
    url: String,
    message: Option<String>,
    token: Option<String>,
) -> anyhow::Result<()> {
    let connect_url = match (&token, url.contains('?')) {
        (Some(_), _) if !url.contains("token=") => {
            format!("{url}?token={}", token.clone().unwrap_or_default())
        }
        _ => url.clone(),
    };
    let (mut ws, _) = tokio_tungstenite::connect_async(&connect_url).await?;
    match message {
        Some(text) => {
            ws.send(Message::Text(text.into())).await?;
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
            while let Ok(Some(msg)) = tokio::time::timeout_at(deadline, ws.next()).await {
                match msg? {
                    Message::Text(t) => println!("{t}"),
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Ok(())
        }
        None => {
            // interactive: stdin lines out, frames in
            let (mut sink, mut stream) = ws.split();
            use tokio::io::AsyncBufReadExt;
            let stdin = tokio::io::BufReader::new(tokio::io::stdin());
            let mut lines = Box::pin(futures::stream::unfold(stdin, |mut stdin| async move {
                let mut buf = String::new();
                match stdin.read_line(&mut buf).await {
                    Ok(0) => None,
                    Ok(_) => Some((buf.trim().to_string(), stdin)),
                    Err(_) => None,
                }
            }));
            loop {
                tokio::select! {
                    line = lines.next() => match line {
                        Some(line) => {
                            if line == ":q" { break; }
                            if sink.send(Message::Text(line.into())).await.is_err() { break; }
                        }
                        None => break,
                    },
                    msg = stream.next() => match msg {
                        Some(Ok(Message::Text(t))) => println!("{t}"),
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Err(e)) => anyhow::bail!("{e}"),
                        _ => {}
                    },
                }
            }
            Ok(())
        }
    }
}

/// Map engine error codes to actionable hints (DX polish).
fn hint_for(code: &str) -> Option<&'static str> {
    match code {
        "quota_exceeded" => {
            Some("raise --max-channels or upgrade: relay billing is configured engine-side")
        }
        "unauthorized" => Some("pass --token matching the engine's --token"),
        "no_such_channel" => Some("create it first: PUT /api/channels/{name}"),
        "inline_secret_disabled" => {
            Some("start the engine with --allow-inline-secret, or use a signing profile")
        }
        _ => None,
    }
}

async fn get_json(
    url: &str,
    path: &str,
    token: &Option<String>,
) -> anyhow::Result<(reqwest::StatusCode, serde_json::Value)> {
    let mut req = reqwest::Client::new().get(format!("{url}{path}"));
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await?;
    let status = resp.status();
    let payload: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    Ok((status, payload))
}

/// `relay doctor`: fast diagnosis of the common setup failures.
async fn doctor(url: &str, capture_url: &str, token: &Option<String>) -> anyhow::Result<()> {
    let mut ok = true;

    // 1) engine reachable?
    match reqwest::Client::new()
        .get(format!("{url}/healthz"))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Err(e) => {
            println!("✗ engine unreachable at {url}: {e}");
            println!("  hint: start it with `relay serve`");
            ok = false;
        }
        Ok(resp) => {
            let version = resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v["version"].as_str().map(str::to_string))
                .unwrap_or_else(|| "?".into());
            println!("✓ engine reachable (version {version})");

            // 2) auth?
            match get_json(url, "/api/billing/plans", token).await {
                Ok((st, _)) if st.is_success() => println!("✓ control-plane auth OK"),
                Ok((st, _)) if st.as_u16() == 401 => {
                    println!("✗ unauthorized — pass --token matching the engine's --token");
                    ok = false;
                }
                Ok((st, v)) => {
                    println!("⚠ unexpected response {st}: {}", v);
                    ok = false;
                }
                Err(e) => {
                    println!("✗ api call failed: {e}");
                    ok = false;
                }
            }

            // 2.5) readiness (durable-layer probe) + version skew
            match get_json(url, "/readyz", token).await {
                Ok((st, _v)) if st.is_success() => println!("✓ readyz ok"),
                Ok((st, v)) => {
                    println!("⚠ readyz {}: {}", st, v);
                    ok = false;
                }
                Err(e) => println!("⚠ readyz probe failed: {e}"),
            }

            // 3) capture plane up?
            match reqwest::Client::new()
                .get(format!("{capture_url}/c/__doctor__"))
                .timeout(std::time::Duration::from_secs(3))
                .send()
                .await
            {
                Ok(r) if r.status().as_u16() == 404 => println!(
                    "✓ capture plane responding at {capture_url} (unknown-channel 404 is expected)"
                ),
                Ok(r) => println!("⚠ capture plane answered {} for a probe", r.status()),
                Err(e) => println!("✗ capture plane probe failed at {capture_url}: {e}"),
            }
        }
    }

    if ok {
        println!("all checks passed");
        Ok(())
    } else {
        anyhow::bail!("doctor found problems")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    Cli::parse().command.run().await
}
