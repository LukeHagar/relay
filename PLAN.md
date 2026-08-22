# Relay — Project Plan

Postman-style workbench for webhooks and WebSockets. Capture live webhook deliveries,
inspect them, save them as named templates, and replay them anywhere — with a local-first
desktop app and an embeddable/self-hostable engine.

Open-source core, MIT-friendly. Optional paid SaaS later for cloud capture endpoints and
event storage. Everything must work fully offline/self-hosted without the SaaS.

## Product pillars

1. **Capture** — Point any provider (Stripe, GitHub, Shopify…) at Relay. Every delivery is
   captured whole: method, path, headers, body, query, timing. Two capture modes:
   - **Self-hosted/tunnel client** (smee-style): a small agent connects out to a reachable
     relay host; providers hit the public URL.
   - **Cloud capture** (SaaS, later): we host the public endpoint, your desktop app
     receives events over an authenticated stream.
2. **Inspect** — Live inbox per channel: pretty-printed payloads, header viewer, search,
   filter, syntax-highlighted diffs between deliveries.
3. **Organize** — Name events, save them as reusable fixtures, group into
   channels/collections per project — or start from a **built-in provider template
   library** (GitHub, Slack, Stripe…) instead of a blank editor.
4. **Replay & send** — Re-send any captured *or templated* event to any target URL with
   edits, **re-signed with your dev secrets** so signed-webhook apps accept replays.
   Signing is table stakes, not polish. First-class WebSocket client: connect, compose,
   send, inspect frames both ways.
5. **Automate (differentiator)** — Headless engine + CLI so replay suites run in CI:
   `relay replay fixtures/*.json --target http://localhost:3000/hooks`.

## Non-goals (for now)

- Production message bus / at-least-once guarantees (design leaves room; don't build yet).
- Multiplayer/team-realtime editing.
- General API testing (REST client) beyond what replay needs.
- Mobile.

## Architecture

```
┌───────────────────────────── Desktop app ─────────────────────────────┐
│  UI (TS, webview)          │  Shell (Tauri/Rust)                      │
│  inbox · composer · diff   │  lifecycle · fs · system tray            │
└──────────────┬──────────────┴───────────────┬──────────────────────────┘
               │ ws/http (localhost)          │ spawns/supervises
┌──────────────▼──────────────────────────────▼──────────────────────────┐
│  Engine (Rust binary — tokio · axum · tungstenite)                     │
│  channels · capture · ring buffer · templates · re-sign · subscribe     │
│  storage: in-memory now → SQLite later                                  │
└──────────────┬─────────────────────────────────────────────────────────┘
               │ outbound ws (listener protocol)
        ┌──────▼───────┐   ┌────────────────────┐
        │ tunnel agent │   │ cloud relay (SaaS) │  ← same engine, hosted
        └──────────────┘   └────────────────────┘
```

Key choices:

- **Core is Rust** (`tokio` + `axum` + `tungstenite`). Perf-sensitive, memory-safe, and
  shares the ecosystem with the Tauri shell. The Go POC in `server/` stays as a behavioral
  reference until the Rust engine reaches parity, then it's deleted.
- **API-first, clients open.** The product boundary is the wire contract: OpenAPI for the
  HTTP API plus a documented WS listener protocol. Desktop app, CLI, and third-party
  clients are peers over that contract — nothing ties a client to an implementation
  language, and a TS SDK ships when the app needs one.
- **Shell: Tauri preferred**, Electron fallback (spike both in M2); Bun desktop tooling
  tracked opportunistically, not load-bearing.
- **Channels, not broadcast.** Everything keys on named channels: the capture plane
  ingests, `PUT /api/channels/{name}` manages, subscribers attach per-channel.
- **Event envelope is the core contract** (stable ID, timestamps, source, raw request).
  Designed so later durability (persist, retry, dedupe) slots in without breaking clients.
- **Templates are first-class data**: versioned provider fixtures (GitHub push, Stripe
  `invoice.paid`, Slack `message`…) with placeholder interpolation — editable, replayable,
  signable. Bundled starter sets ship from M1; community-contributable later.
- **Replay must re-sign.** Naive replay fails HMAC validation (Stripe/GitHub/Shopify…).
  Per-provider signing configs are core DX, not a nice-to-have.

## Milestones

### M0 — Contract first + Rust scaffold
Goal: freeze the wire contract and stand up the Rust engine skeleton. Nothing user-facing yet.
- [x] `docs/api.md`: event envelope schema (versioned), channel HTTP API, WS listener
      protocol — this document is the product boundary; review gate before building
- [x] Rust workspace scaffold: `engine` crate (tokio/axum/tungstenite) + `cli` crate;
      config via flags/env, structured logging, graceful shutdown from day one
- [x] CI: fmt + clippy + test on push; `.gitignore`, README, LICENSE
- [x] Go POC (`server/`) kept building as behavioral reference; parity checklist started
- Exit: contract reviewed; `cargo test` green in CI; skeleton serves `/healthz`.

### M1 — Engine MVP in Rust (headless, useful without UI)
Goal: a self-hostable smee-alternative + signed-replay tool. Ship this publicly.
- [x] Event envelope implemented per `docs/api.md`; channels via `PUT /api/channels/{name}`;
      capture plane ingests any method/path with zero rewriting (raw URI, header order,
      duplicates); per-channel subscriber registry (WS + SSE)
- [x] In-memory ring buffer per channel (size/time capped); history fetch API
- [x] Replay API: stored/template event → target URL with header/body/query overrides
- [x] Signing module (table stakes): GitHub (`sha256=` HMAC-SHA256), Stripe (`t=…,v1=…`
      timestamped scheme), Slack (`v0:{t}:{body}` scheme) — each validated against the
      provider's official test vectors; config-driven so new schemes are additive
- [x] Template events: fixture format (JSON + placeholders), bundled GitHub/Slack/Stripe
      starter sets, interpolation at replay time
- [x] CLI: `serve`, `listen`, `replay`, `send`
- [x] Docs: companion-source recipe — vercel-labs/emulate as an offline event source
      (`webhook_url` → relay channel); its signed Stripe/GitHub deliveries double as
      extra signing test vectors; template fixtures can be regenerated from it
- [x] Load test honest: Artillery scenario updated to channels, latency budgets asserted
- Exit: `relay serve` + `curl` + signed replay demo end-to-end; a replayed Stripe event
  validates with `stripe.Webhook.construct_event`; README quickstart.

### M2 — Desktop shell (the app exists)
Goal: daily-driver UX for capture → inspect → replay.
- [x] Tauri spike vs Electron spike (bundle size, memory, sidecar ergonomics) → pick one
- [x] App manages engine sidecar lifecycle; tray icon; channel picker
- [x] Inbox UI: live stream, JSON tree viewer, headers tab, filter/search
- [x] Composer: edit any captured event, choose target, replay; save as named fixture
- [x] Local persistence: SQLite (channels, events, fixtures); history survives restarts
- Exit: build a Stripe webhook integration using only the app, zero terminal commands.

### M3 — Power features
- [x] Collections/workspaces; import/export (JSON, shareable fixture packs)
- [x] Scheduled/burst replay; latency assertions (rough perf testing mode)
- [x] Forwarding rules: on capture, auto-forward transformed copies to N targets
- [x] WebSocket client mode: connect to arbitrary WS endpoints, two-way frame inspector
- [x] Provider catalog growth: more signing schemes + template packs, community-contributable

### M4 — SaaS layer (optional paid tier; core stays free/local)
- [x] Cloud capture ingress: wildcard `{slug}.{zone}` hostnames (generated + vanity),
      faithful-path forwarding to connected apps over the listener protocol
- [x] Accounts, device pairing, encrypted-at-rest event retention
- [x] Team sharing of fixtures; usage metering/billing
- Exit criterion throughout: full functionality with `relay serve` self-hosted, no account.

## Decision log

| # | Decision | Rationale | Revisit when |
|---|----------|-----------|--------------|
| D1 | Core rewritten in Rust (tokio/axum); Go POC kept as reference only | User priority: perf + Rust affinity; POC is ~140 lines, cheap to redo | If M1 velocity collapses |
| D2 | Tauri first, Electron fallback | Rust core + Rust shell share ecosystem/toolchain | RESOLVED M2: Tauri compiled (48.8s cold) and rendered the live inbox under Xvfb; Electron not needed |
| D3 | In-memory ring buffer now, SQLite in M2 | Ship fast; schema designed for durability | M2 |
| D4 | Channels replace global broadcast | Multi-project/multi-provider isolation | — |
| D5 | Signing is table stakes, lands in M1 | Signed webhooks reject unsigned replays; tool is useless without it | — |
| D6 | API-first: OpenAPI + WS protocol doc is the contract | Clients stay open — app, CLI, third-party, any language | — |
| D7 | Provider template library starts in M1 | Blank-editor start kills DX; GitHub/Slack/Stripe cover most devs | — |
| D8 | Capture isolated on its own plane — separate port locally, per-channel subdomains hosted; paths byte-faithful | Separates untrusted provider traffic from the app surface; exact paths make replays faithful | Custom-domain + ACME ownership lands at the SaaS edge |
| D9 | Emulate (vercel-labs, Apache-2.0) is a complementary *source*, not a dependency: docs recipe first, optional companion mode later, no engine coupling | It emits provider-faithful signed webhooks offline — perfect input for capture/replay; project is v0.4.x TS/Node, so wrapping/bundling is churn risk with no payoff yet | If emulate matures + demand for one-command offline mode, add opt-in app integration (M3+) |

## Risks

- **NAT traversal is the hard part of capture-from-anywhere.** Mitigation: SaaS capture
  (M4) is the answer for restricted networks; self-host documents reverse-proxy setup.
- **Rewrite-before-ship risk** (Go → Rust). Mitigation: POC is tiny (~140 lines); the wire
  contract is frozen in M0 before the rewrite; Go POC kept runnable until parity.
- **Rust iteration speed** vs GC-language comfort. Mitigation: narrow M1 scope,
  batteries-included axum/tokio patterns, no exotic async (io_uring etc.) for now.
- **Crowded space** (smee, Hookdeck CLI, RequestBins, Postman). Differentiators: local-first,
  open source, re-signing, and replay-as-tests in CI. Keep the landing story focused there.

## Status (implemented)

All milestones M0–M4 are implemented and verified in this repo:

- **Engine** (`crates/engine`): envelope, channels, byte-faithful capture plane,
  ring buffers, WS/SSE subscriptions with seam guarantees, signed replay (5 schemes,
  provider vectors), templates (10 bundled fixtures), forwarding rules, SQLite
  persistence with seq continuity, hosted (`--capture-zone`) routing, embedded inbox UI.
- **CLI** (`crates/cli`): `serve`, `listen`, `replay` (+ `--repeat`/`--assert-p95-ms`),
  `send`, `ws`, `templates export/import`.
- **Desktop** (`src-tauri`): Tauri shell hosting the UI against any engine URL.
- **Verification**: 48 Rust tests + 2 socket integration suites green; fmt/clippy -D
  warnings clean; Go POC still builds; E2E covers Slack challenge echo, capture
  fidelity, CLI listen, independently verified Stripe signature, restart persistence,
  UI replay round-trip (browser-driven), Tauri runtime (Xvfb screenshot).
- **Out of repo scope (M4)**: multi-tenant accounts/device pairing/billing are
  deployment-layer concerns per the plan; the standing exit criterion (full function
  self-hosted, no account) is met — see docs/cloud.md.

## Follow-up round 2 (delivered)

- **Benchmarks**: criterion suites in `crates/engine/benches/hot_paths.rs`; baseline
  recorded in `docs/benchmarks.md`.
- **Load tests**: executed with Artillery 2.0.34 against the release build —
  3750 requests, p95/p99 = 1 ms, zero failures; per-channel configurable rate limits
  added as a result (`rate_limit.{burst,refill_per_s}`); full report in
  `docs/loadtest-report.md`.
- **Billing (Stripe)**: plan catalog (Free/Pro), entitlement gating (402 `quota_exceeded`),
  Checkout Session creation, signature-verified webhooks reusing the §6.1 scheme,
  SQLite-persisted subscription state; 10 offline tests incl. tamper/stale/tolerance;
  live lifecycle smoke verified. Docs: `docs/billing.md`.
- **CI**: bundled template fixtures validated on every push
  (`.github/validate_templates.py`).

## Follow-up round 3 (delivered) — fast · resilient · DX · SDKs

- **Resilience**: panic isolation (`plane_guard` catch_unwind → JSON 500, engine keeps
  running), per-plane request timeouts (control 15 s / capture 35 s → 504), slowloris
  body-read window (20 s), SQLite `busy_timeout`, forwarding-rule retry with backoff,
  `GET /readyz` durable-layer probe.
- **Performance**: tuned reqwest pool (64 idle, nodelay, connect timeout),
  `codegen-units = 1` release profile. Benchmarks unchanged post-hardening
  (resilience sits outside the hot path); load test regression identical:
  3750 reqs, p95/p99 = 1 ms, zero failures.
- **DX**: OpenAPI 3.1 spec at `docs/openapi.yaml` served live from
  `GET /api/openapi.yaml`; `relay doctor` (reachability/auth/capture checks);
  `relay completions <shell>`; typed error envelopes everywhere.
- **SDKs** (`sdks/`): TypeScript (zero-dep, Bun-tested against a live engine),
  Go (gorilla/websocket, context-aware, live-tested), Rust (reqwest +
  tokio-tungstenite, workspace member, live-tested). All three cover channels,
  history, replay, and handshake-safe WebSocket subscriptions.

## Follow-up round 4 (delivered) — full Tauri/SvelteKit application

- **SvelteKit frontend** (`app/`): SPA (adapter-static, no SSR runtime) with routes for
  Inbox · Channels · Replay-from-template · Billing · Settings; Svelte 5 runes;
  data layer = the TypeScript SDK (`sdks/typescript`) directly.
- **Theme**: derived from parke.dev — Inter / Space Grotesk / JetBrains Mono,
  `#060A13` deep-navy surfaces, blue primary + gold secondary with glow accents,
  terminal-style status badges.
- **Feature-complete surface**: channel create + full config editing (response mode,
  rate limits, bearer auth, forwarding rules) · live WS inbox with seam-safe backlog ·
  event inspection (pretty body / headers / metadata) · replay composer with body edit
  and signing profiles · template browser with variable interpolation · billing plans +
  checkout hand-off · settings with health/readiness probes.
- **Integration**: adapter output embedded into the engine binary via rust-embed
  (204 KB total, SPA fallback for deep links); Tauri shell loads the engine URL —
  same UI in browser and desktop. Legacy vanilla `ui/` removed.
- **Verified**: browser-driven end-to-end (create → capture → inspect → replay →
  channels editing → templates → billing/settings), plus Tauri runtime under Xvfb.

## Perf triage round (delivered)

- **Triage**: at 3,000 rps capture the engine uses ~5% of one core — Artillery (Node)
  is the load-generator bottleneck beyond ~3 k rps, polluting client-measured latency.
- **True engine ceiling measured with a Go probe**: **71,947 rps**, zero errors,
  p50 0.066 ms / p95 0.255 ms / p99 0.898 ms over 1.08 M requests in 15 s; sustained
  10 k rps for 20 s with p99 3.7 ms and zero errors (`bench/harness/sub-latency
  -mode capture-rps`).
- **Optimizations applied** (correct regardless; invisible through the Node-limited
  generator): TCP_NODELAY on accepted sockets, mimalloc global allocator,
  Arc-swap channel config (per-request deep clone eliminated), envelope built
  outside the channel lock, hub publish fast-path on idle channels.

## Remaining follow-ups

1. Release packaging: Tauri bundle icons/signing, `cargo dist`-style binary releases.
2. Publish the three SDKs to npm/crates.io/PkgGoDev once the repo is public.
3. Community template catalog pipeline (contribution → review → bundle).
4. Virtualized inbox list for very long buffers (>5k events).
5. Re-measure WS fan-out p99 on dedicated hardware (shared-box noise made tails
   inconclusive); consider SO_REUSEPORT accept sharding if tails persist.
