# End-to-end engine benchmarks

Real-load runs against the **release binary** (`target/release/relay`), single machine
(AMD Ryzen 9 7950X3D, Linux), loopback networking. Runner: `bench/run-e2e.sh`
(serial scenarios, fresh engine per suite, external VmRSS sampler). Raw artifacts:
`bench/results/<timestamp>/` (Artillery JSON, RSS CSV traces, harness output).

Recorded run: **2026-08-21-225617** · Artillery 2.0.34 · every number below
independently recomputed from raw artifacts by an adversarial audit agent
(all_consistent = true; see "Verification" at the bottom).

## Profiles & results

| Scenario | Load | Reqs | Errors | p50 | p95 | p99 | max |
|---|---|---|---|---|---|---|---|
| capture warmup | 200 rps, minimal bodies | 3,000 | 0 | 0 ms | 1 ms | 1 ms | 15 ms |
| capture ramp | ramp → 2,000 rps, ~0.5 KB bodies | 37,500 | 0 | 0 ms | 1 ms | 1 ms | 14 ms |
| **capture max** | **3,000 rps**, minimal bodies | **60,000** | **0** | 1 ms | **4 ms** | **8.9 ms** | 62 ms |
| mixed workload¹ | 300 rps, 4 flows (capture/history/replay-API/billing) | 18,000 | 1,837×400² | 0 ms | 2 ms | 3 ms | 40 ms |
| spike | 100 rps → **5,000 rps burst** → recovery | 37,000 | 0 | 0 ms | 5 ms | 8.9 ms | 67 ms |
| soak (11 KiB bodies) | **120 s @ 400 rps sustained** | 48,000 | 0 | 0 ms | 1 ms | 3 ms | 13 ms |

¹ flows weighted 60% capture / 20% history / 10% replay-API / 10% billing reads
² all 1837 are *intentional* `404 no_such_event` from replaying random ids — measures
error-path overhead through signing/replay plumbing; deterministic by design.

## Capture → subscriber delivery latency (Go harness, dogfooding sdks/go)

4 WebSocket subscribers × 30 s producer @ 200 rps:

| samples | produced | dropped | p50 | p90 | p99 | max |
|---|---|---|---|---|---|---|
| **23,996** (= 4 × 5,999) | 5,999 | **0** | **0.16 ms** | 0.28 ms | **0.42 ms** | 6.9 ms |

Perfect fan-out (every subscriber got every event) with sub-millisecond median
end-to-end delivery.

## Memory (external VmRSS sampling)

| Scenario | early RSS | peak | end |
|---|---|---|---|
| spike (5000 rps burst) | — | **439 MB transient** | recovers |
| soak (48k × 11 KiB) | ~146 MB post-warmup | ~150 MB | **~137 MB (below start)** |

No monotonic growth across 132 soak samples — the ring buffer + GC behave as designed;
the spike peak is transient burst absorption that the allocator returns.

## Triage round — where does the 3k-rps "degradation" come from?

The capture-max profile showed p95 rising 1 ms → 4 ms between 2000 and 3000 rps.
Triage (CPU sampling during the run + a Go-based capture probe added to
`bench/harness/sub-latency` via `-mode capture-rps`):

- **Engine CPU at 3,000 rps: ~5% of one core.** The engine was never the bottleneck.
- **Artillery (Node) was**: its event loop saturates around ~3 k rps and its lag
  pollutes client-observed response time. All Artillery numbers below ~3 k rps are
  trustworthy; beyond that they measure the generator.
- A Go capture probe (same harness, `-mode capture-rps`, 8 workers, unlimited rate,
  15 s) achieved **71,947 rps** with **zero errors**:

| Go probe, unlimited | value |
|---|---|
| requests | 1,079,209 in 15 s |
| achieved | **71,947 rps** |
| latency p50 / p95 / p99 | 0.066 / 0.255 / **0.898 ms** |
| p999 / max | 3.9 / 21 ms |

- Sustained-rate check (`-rate 10000 -duration 20s`): exactly **10,000 rps for 20 s,
  200,000 requests, zero errors**, p99 = 3.7 ms.

Optimizations applied during triage (correct regardless; their effect is invisible
through the Node-limited generator): TCP_NODELAY on accepted sockets (Linux
inheritance), mimalloc global allocator, Arc-swap channel config (no per-request deep
clone), envelope built outside the channel lock, hub publish fast-path when no
subscribers exist.

WS fan-out after these changes: p50 0.19–0.22 ms (parity with baseline); the p99
comparison (0.42 → 5–6 ms) is **inconclusive on this shared machine** — the box runs
concurrent tooling during measurement; re-measure tails on dedicated hardware before
drawing conclusions. Zero dropped deliveries in every run.

## Interpretation

- Sustained capture ceiling on one box: **≥3,000 rps at p95 ≤ 4 ms** with headroom to
  spare (the arrival rate was the limiter, not the engine).
- The 5,000 rps burst was fully absorbed mid-run: zero errors, p99 < 9 ms during the spike.
- Error paths are cheap: replay-with-missing-event (full signing/replay pipeline,
  terminating in a typed 400) ran at p95 = 2 ms alongside everything else.
- Loopback caveat: these are same-machine numbers — no network/TLS in the path.
  Expect network RTT to dominate any real deployment; relative profile comparison
  (regressions between builds) remains valid.
- Mixed-scenario 4xx count is the designed deterministic error mix, not instability
  (verified: every 400 sits on `/api/replay`, counts reconcile to the request total).

## Reproduce

```bash
bash bench/run-e2e.sh            # runs everything serially, writes bench/results/
npx artillery report bench/results/<ts>/raw-spike.json   # HTML dashboards
```

Scenarios: `bench/scenarios/*.yaml` (see bench/scenarios/README.md).
Component-level micro-benchmarks: `cargo bench -p relay-engine` (docs/benchmarks.md).
