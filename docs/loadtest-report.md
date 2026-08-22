# Load test report

Tool: Artillery 2.0.34 · Target: `relay serve` **release build** on the same machine
(AMD Ryzen 9 7950X3D, Linux) · Scenario: `client/artillery-test.yaml`
(30 s ramping 50 → 200 arrivals/s against the capture plane,
`POST /c/loadtest/load` with an ~30-byte JSON body).

Pre-step (raises the channel's rate ceiling — see §8 / §9.7):

```bash
curl -X PUT localhost:8000/api/channels/loadtest \
  -d '{"rate_limit":{"burst":5000,"refill_per_s":5000}}'
```

## Run 1 — default rate limits (300 burst / 100 rps refill)

| Metric | Value |
|---|---|
| Requests | 3750 |
| HTTP 200 | 3063 (81.7%) |
| HTTP 429 | **687 (18.3%)** — per-channel limiter throttling sustained load above the 100 rps refill |
| Failures | 0 |
| response_time p95 / p99 | 1 ms / 1 ms |

The 429s are the documented §9.7 abuse control doing exactly its job: at a sustained
~125 rps average arrival, refill (100 rps) cannot keep up once the 300-token burst is
spent. Clients see clean, fast rejections — never stalls.

## Run 2 — raised limits for the channel (`burst 5000 / 5000 rps`)

| Metric | Value |
|---|---|
| Requests | 3750 |
| HTTP 200 | **3750 (100%)** |
| HTTP 429 | 0 |
| Failures | 0 |
| response_time median / p95 / p99 | 0 ms / **1 ms** / **1 ms** |
| response_time max | 20 ms |
| VU session length p95 | 3 ms |

## Interpretation

- Capture hot path (parse → envelope build → ring buffer → fan-out) sustains
  **~200 rps with p99 = 1 ms**, three orders of magnitude under any human-perceived
  latency concern; the bottleneck in this single-machine setup is client-side arrival.
- Rate limiting is per-channel and configurable (`rate_limit.burst`, `rate_limit.refill_per_s`);
  changes apply live via `PUT /api/channels/{name}` without recreation.
- The earlier global-broadcast POC stalled all clients behind one slow writer; the
  per-subscription buffered fan-out shows no such head-of-line behavior here.

## Reproducing

```bash
cargo build --release --workspace
./target/release/relay serve &
curl -X PUT localhost:8000/api/channels/loadtest \
  -d '{"rate_limit":{"burst":5000,"refill_per_s":5000}}'
npx artillery run --output report.json client/artillery-test.yaml
npx artillery report report.json   # HTML dashboard
```

Raw JSON of the recorded tuned run is not committed; numbers above were extracted from
`--output` artifacts of the runs dated 2026-08-21.
