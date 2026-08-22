# Artillery scenarios

Run with the benchmark runner (starts `./target/release/relay serve`, pre-creates
`bench-capture`, `bench-mixed`, `bench-replay-src`, `bench-replay-dst` with
`rate_limit {burst: 50000, refill_per_s: 50000}`, then runs each scenario with
`npx artillery run --output <raw.json> <scenario>`, followed by the Go latency
harness).

All scenarios target the **capture plane** (`http://127.0.0.1:8001`) as their
Artillery `config.target`; control-plane (`http://127.0.0.1:8000`) API calls are
made via absolute URLs inside flows where a flow needs them. Channels are
pre-created by the runner; scenarios never create channels.

## mixed-workload.yaml

Realistic developer session against BOTH planes: one phase, four weighted flows.

Design choice: **single 60 s phase at `arrivalRate: 300` with per-scenario
`weight`s** (6 / 2 / 1 / 1), not separate phases per flow type. Artillery's
phase arrival rate is test-wide and arrivals are distributed across scenarios
proportionally to weight, so this produces one true 300 req/s total-arrival
stream with interleaved request types — which is what "mixed workload" means;
back-to-back homogeneous phases would measure four isolated micro-benchmarks
instead (different cache/buffer dynamics, no contention mixing) and would also
need per-phase rate arithmetic to hit the same 300/s total.

| Flow | Weight | Share | Request |
|------|--------|-------|---------|
| A `flowA-capture-github-push` | 6 | ~60% | POST `/c/bench-mixed/github` |
| B `flowB-control-list-events` | 2 | ~20% | GET `:8000/api/channels/bench-mixed/events?limit=50` |
| C `flowC-replay-miss` | 1 | ~10% | POST `:8000/api/replay` |
| D `flowD-billing-plans` | 1 | ~10% | GET `:8000/api/billing/plans` |

**Payload sizing (flow A):** inline JSON body of **11,179 bytes rendered**
(verified), matching `bench/payloads/github-push.json` (11,530 bytes, ±3%).
Inline rather than a CSV/payload reference because the body must carry two
templated uniqueness fields (`before`/`after` shas plus the first commit id get
`$randomNumber`), which a verbatim fixture file cannot provide. Structure
mirrors a real GitHub push webhook: repository metadata + 12 commits with
realistic multi-line messages.

**Flow C is deliberately an error path.** It sends a schema-valid replay
request (`target_url` pointing at the capture plane, required by the API) with
a random nonexistent `event_id`, so it almost always returns **404
`no_such_event`**. This intentionally measures *replay API overhead including
the error/lookup path*, NOT successful replay delivery latency. Expect its
latency numbers to be dominated by request parse + event lookup + error
serialization; do not read them as end-to-end replay cost.

**Gate:** `config.ensure http.response_time.p95 < 40 ms` over all flows.
Smoke-validated at full rate: ~300 req/s achieved, weights split
59.7/21.3/10.9/8.1%, overall p95 = 4 ms locally, flow C responses 100% 4xx as
designed.

## Other scenarios

Add one section per scenario file here documenting target plane, phase/rate
design, payload sizes, and any intentional error paths.

## capture-throughput-{1-warmup,2-ramp,3-max}.yaml

Sustained capture-ceiling probe for channel `bench-capture`, split into THREE
files that the runner executes serially (one `raw.json` each):

| File | Phase | Offered load | Body | Endpoint |
|------|-------|--------------|------|----------|
| `capture-throughput-1-warmup.yaml` | 15 s steady | 200 req/s | minimal (~15 B rendered; fixture: 11 B) | POST `/c/bench-capture/warm` |
| `capture-throughput-2-ramp.yaml` | 30 s linear ramp | 500 → 2000 req/s | stripe-invoice-shaped (~550 B rendered; fixture: 528 B) | POST `/c/bench-capture/invoice` |
| `capture-throughput-3-max.yaml` | 20 s steady | 3000 req/s | minimal (~15 B) | POST `/c/bench-capture/max` |

**Why three files instead of one YAML with three phases:** Artillery phases
are global — scenario selection is a weighted pick per spawned virtual user
(verified in artillery 2.0.34 `lib/core/runner.ts`: one picker built from
`script.scenarios`, driven by `config.phases`). Distinct flows therefore
cannot be bound to distinct phases inside a single file; a 3-phase/3-scenario
file would blend all three endpoints and body sizes into every phase. Serial
single-phase files keep each profile exact AND make each `raw.json`
phase-pure (Artillery aggregates histograms over a whole file run, so a
multi-phase file would blur p95/p99 across phases).

**Payloads:** processor-free inline JSON, uniquified per request with
`{{ $randomNumber }}` id fields so every event body is distinct (no
dedupe/cache artifacts). Sizes match `bench/payloads/minimal.json` (11 B) and
`bench/payloads/stripe-invoice.json` (528 B); the invoice body mirrors the
fixture's structure (`invoice.paid` event with one invoice line).

**Gate (all three):** `config.ensure` fails the run if
`http.response_time.p95 >= 25 ms` or `p99 >= 100 ms`.

**Reading the results as a ceiling test:** warmup establishes a clean
baseline at trivial load; the ramp shows whether latency stays flat as load
rises 4x with ~40x larger bodies; max is the probe — small bodies isolate the
capture path from serialization cost. If the max phase holds its ensure gate
with zero 429s, capture sustains ≥ 3000 events/s on this host/build; if p95
or p99 blows past the gate, ~3000 req/s is the observed ceiling.

**429s mean limits were NOT raised.** All channels run with
`rate_limit {burst: 50000, refill_per_s: 50000}`; the highest offered rate is
3000 req/s, two-plus orders of magnitude under both burst and refill, so any
`http.codes.429` in a raw output indicates the token bucket was not actually
sized up (runner/config problem), NOT that the server is at capacity. Never
read 429s here as headroom being exhausted.

## Interpreting Artillery raw output (`--output <raw.json>`)

- The raw JSON is an aggregate statistics blob for the entire file run:
  look under `aggregate.counters` and `aggregate.summaries`.
  - `http.request_rate` — achieved mean req/s (compare against offered
    arrivalRate; a shortfall means the client or a keep-alive limit is the
    bottleneck, not the relay).
  - `http.codes.{200,404,429,...}` — response-code counters.
  - `summaries.http.response_time` — `{min, max, mean, p50, p95, p99}` in ms;
    these are what the `ensure` gate is evaluated against.
  - `vusers.created` / `vusers.failed` — should equal requests issued;
    failures usually surface as connection errors rather than HTTP codes.
  - `http.downloaded_bytes` / `http.uploaded_bytes` — sanity-check total
    bytes vs expected (rate × duration × body size).
- Pretty view: `npx artillery report bench/results/<raw>.json` renders an
  HTML report with the same numbers charted.
- A failed `ensure` prints an `[ensure]` summary and exits non-zero while the
  raw.json still contains full metrics — use the metrics, not just the exit
  code, to say *by how much* a gate was missed.
- Because each throughput profile is its own file/run, each raw.json's
  histograms are phase-pure; compare p95/p99 ACROSS the three raw files to
  get per-phase resolution, which a single multi-phase file cannot give you.
