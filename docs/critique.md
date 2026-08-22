# Adversarial review — UI & API DX (workflowz run, 2026-08-21)

Method: 6 personality-diverse finders (Theo-Browne ×2 lenses, Matt-Pocock ×2,
infrastructure skeptic, a11y/visual critic) → 57 raw findings → deduped → every
finding re-attacked by a hostile verifier instructed to REFUTE (default: refuted
when unsure) by reading the current code. 44 confirmed, 13 refuted. All confirmed
high/medium items were then fixed in this pass; lows selectively.

## Confirmed & fixed

### UI
- **WS auth + reconnect dedup + channel-switch race** (high): the live socket never
  sent the token, reconnects replayed the whole buffer into a dedup-less list, and a
  stale socket could write into the new channel's list. `subscribeLive` now sends
  `?token=`, resumes from `last_seq`, dedupes by bounded id-set, and generation-guards
  every message.
- **Sidebar /replay 404** (med): dead nav entry removed from the shell.
- **Dialog swallowed failures** (med): creation errors now render inside the dialog.
- **Keyboard nav** (med): ↑/↓ moves selection through the inbox (listbox pattern).
- **A11y** (high/med): `role="status"` live region for capture/count, `role="tablist"`
  with `aria-selected` tabs + visible active styling, `aria-selected` + accent-bar
  selection on rows, labeled ✕ buttons, dialog `aria-labelledby` + error slot.
- **Daily-loop friction** (low): relative timestamps with ISO on hover, copy-event-id
  button.
- **Debug chrome** (low): hardcoded `ONLINE` badge now reflects real connection state.

### API / engine
- **WS frames violated `v:1`** (high): every `ServerFrame` now carries `v: 1`
  (constructors enforce it; tests assert it).
- **Replay killed by the plane guard** (high): `/api/replay` is exempt from the 15 s
  control timeout (75 s budget) so `timeout_ms ≤ 60 s` is honored and delivery state
  stays unambiguous.
- **Forwarding retry never existed** (high): the Transient/Permanent classification
  was dead code — the retry loop is now real (2 attempts, 200 ms backoff) and the
  whole delivery is bounded by a 10 s timeout so stalled targets can't leak tasks.
- **Durable-history failures returned 200 + empty list** (med): now a typed
  `500 history_unavailable` envelope.
- **Persist failures invisible** (med): capture acks carry `x-relay-persisted: false`
  and the engine counts failures (`persisted_failures`).
- **`?token=` accepted everywhere** (med): query tokens now only honored on `/ws` and
  SSE routes where browsers can't set headers.
- **`--max-channels` documented but missing** (med): flag implemented, enforced with
  `402 quota_exceeded`.
- **SSE `limit` documented but ignored** (med): backlog is clamped now.
- **Static-mode misconfig → 500 `bad_request`** (low): now `internal_error` with an
  honest message.

### SDKs / contract
- **TS**: typed `ChannelDescriptor`/`Plan`/`SubscriptionState` (no more
  `Record<string, unknown>`), exported `ServerFrame` discriminated union,
  `headersToMap` helper, `captureBase` constructor option, `billingCheckout`/
  `health`/`ready` methods, `channel_closed` handled instead of ignored.
- **Rust**: full `ChannelConfig` (static_response/auth/buffer/rate_limit/forward),
  typed `ReplayRequest`/`ReplaySourceWire` (no more untyped `Value`), correct
  `capture_base` derivation (port+1, overridable), `headers_map()` helper.
- **Contract docs**: `openapi.yaml` gained the three missing routes, a sound
  `ReplaySource` discriminator (`required` + `additionalProperties: false`),
  explicit nullability; `docs/api.md` error table rewritten (dead 409 removed,
  402/429/500/503/504 + every real code documented).

## Refuted (with reasons — kept for the record)

| Finding | Why refuted |
|---|---|
| Templates page `vars` typing broken | Runtime evidence contradicts the diagnosis |
| `$state` deep proxies force snapshot casts everywhere | Reproduced Svelte 5.56 behavior; casts not required |
| Settings JSON.parse unsafe spread | Spread over defaults is total, not partial |
| Doctor "checks nothing durable" | readyz probe exists; auth probe on /api/billing/plans is registered & token-aware |
| "OpenAPI can't express header tuples" | False — tuples declared fine; map access is an additive nicety |
| TS SDK missing tsconfig / EOPT hazards | No tsconfig is true; the load-bearing EOPT claim is false |
| No :focus-visible styles | Present in app.css |
| Zero transitions anywhere | Buttons animate; claim over-generalized |
| Radius scale drift / heading font inconsistency | Radius values form a coherent role-based tier scale |

## Deliberately deferred

- Virtualized inbox for >5k-event buffers (500-cap ring keeps worst-case bounded;
  revisit if real usage outgrows it).
- `listen --until <duration>` (only `--max-events` shipped).
- Publishing SDKs to npm/crates.io/PkgGoDev (repo must go public first).
