# Relay

Postman-style workbench for webhooks and WebSockets: capture live deliveries,
inspect them, save them as templates, and replay them anywhere — correctly
signed. Local-first desktop app with an embeddable, self-hostable engine.

- **Plan**: [PLAN.md](PLAN.md)
- **Wire contract (v1)**: [docs/api.md](docs/api.md)

## Status

M0–M4 implemented and verified — see [PLAN.md](PLAN.md) for the delivered scope and
[docs/api.md](docs/api.md) for the wire contract. 50+ tests; CI runs fmt/clippy/test
plus the Go POC reference build.

## Quick start (engine)

```bash
cargo build -p relay-cli
./target/debug/relay serve
# control plane: http://127.0.0.1:8000  — /api/*, /ws, SSE
# capture plane: http://127.0.0.1:8001  — webhook ingestion, any method
```

A capture loop in five lines (grab an `id` from step 3's output; `listen` wants its
own terminal):

```bash
curl -X PUT localhost:8000/api/channels/demo                     # 1. create the capture point
curl -X POST localhost:8001/c/demo/hook -d '{"hello":"relay"}'   # 2. fire a webhook at it
curl localhost:8000/api/channels/demo/events                     # 3. read back what was captured
./target/debug/relay listen demo --from newest                   # 4. terminal 2: live tail
./target/debug/relay replay --target http://localhost:3000/hooks --event <EVENT_ID> --profile gh-local   # 5. resend to your app, re-signed
```

Signing profiles resolve secrets out-of-band via engine config (`--config`), e.g.
`{"signing_profiles": {"gh-local": {"scheme": "github", "secret_env": "GH_DEV_SECRET"}}}` —
see [docs/api.md](docs/api.md).

## Layout

```
crates/engine   engine library: channels, capture, buffers, subscriptions, replay, signing
crates/cli      `relay` binary: serve, listen, replay, send, ws, templates
app/            SvelteKit frontend (parke.dev-themed), embedded in the engine binary
src-tauri       desktop shell (Tauri) hosting the SvelteKit UI
server/         Go POC kept as a behavioral reference (superseded by the Rust engine)
client/         minimal client example
templates/      bundled provider fixtures (GitHub, Stripe, Slack, Shopify, Twilio)
docs/           wire contract (api.md) · benchmarks (micro + e2e load) · billing · cloud
```

## Load test

```bash
curl -X PUT localhost:8000/api/channels/loadtest   # pre-step: the channel must exist
npx artillery run client/artillery-test.yaml       # hammers the capture plane, asserts p95/p99 budgets
```

## Desktop & web app

SvelteKit + Tauri. The engine embeds the built frontend — one binary serves the whole
UI at `/`; the Tauri shell wraps it as a native desktop app.

```bash
cd app && bun install && bun run build   # → app/build (embedded)
cargo build --workspace                  # engine now serves the app
cd src-tauri && cargo build              # optional native shell
```

## SDKs

| Language | Path | Notes |
|---|---|---|
| TypeScript | [`sdks/typescript`](sdks/typescript) | zero deps; Node ≥22 / Bun / Deno / browsers |
| Go | [`sdks/go`](sdks/go) | context-aware, gorilla/websocket only |
| Rust | [`sdks/rust`](sdks/rust) | async (reqwest + tokio-tungstenite), workspace member |

All three expose channel management, history, signed replay, and handshake-safe
WebSocket subscriptions over the same contract as the engine.

## Benchmarks & load tests

```bash
cargo bench -p relay-engine        # criterion hot paths → target/criterion
bash bench/run-e2e.sh              # full e2e suite: 6 profiles incl. spike/soak,
                                   # WS fan-out latency harness, RSS sampling
```

Recorded results: [docs/benchmarks.md](docs/benchmarks.md) (micro) and
[docs/e2e-benchmarks.md](docs/e2e-benchmarks.md) (end-to-end: 3k rps capture at
p95 4ms, 5k rps burst absorbed, 4-subscriber fan-out p99 0.42ms, no-leak soak).
Scenarios live in `bench/scenarios/`.

## Billing

Open-core: Free plan (5 channels, no forwarding) out of the box; Pro via Stripe
Checkout. Configuration and webhook flow: [docs/billing.md](docs/billing.md).
Load-test results against a release build: [docs/loadtest-report.md](docs/loadtest-report.md).

## Development

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
go build ./...   # POC reference must keep building
```
