#!/usr/bin/env bash
# Relay end-to-end benchmark runner.
#
# Runs every load profile serially against a fresh release engine, samples engine
# RSS during each run, and drops raw Artillery JSON + RSS traces into bench/results/.
# Usage: bash bench/run-e2e.sh [results-dir]
set -euo pipefail

RESULTS="${1:-bench/results/$(date +%Y%m%d-%H%M%S)}"
ENGINE="${ENGINE:-./target/release/relay}"
CONTROL="127.0.0.1:8000"
CAPTURE="127.0.0.1:8001"
mkdir -p "$RESULTS"

echo "== building release engine =="
cargo build --release --workspace 2>&1 | tail -1

cleanup() {
  [[ -n "${SRV_PID:-}" ]] && kill "$SRV_PID" 2>/dev/null || true
  [[ -n "${RSS_PID:-}" ]] && kill "$RSS_PID" 2>/dev/null || true
}
trap cleanup EXIT

start_engine() {
  nohup "$ENGINE" serve --control-addr "$CONTROL" --capture-addr "$CAPTURE" \
    >>"$RESULTS/engine.log" 2>&1 &
  SRV_PID=$!
  for _ in $(seq 40); do
    curl -sf "http://$CONTROL/healthz" >/dev/null && return 0
    sleep 0.25
  done
  echo "engine failed to start"; cat "$RESULTS/engine.log"; exit 1
}

rss_sampler() { # $1 = out csv, $2 = label
  (
    echo "t_seconds,rss_kb"
    start=$(date +%s)
    while kill -0 "$SRV_PID" 2>/dev/null; do
      rss=$(grep VmRSS "/proc/$SRV_PID/status" 2>/dev/null | awk '{print $2}')
      [[ -n "$rss" ]] && echo "$(( $(date +%s) - start )),$rss" >> "$1"
      sleep 1
    done
  ) &
  RSS_PID=$!
}

run_scenario() { # $1 = label, $2 = yaml
  local label="$1" yaml="$2"
  echo "== scenario: $label =="
  rss_sampler "$RESULTS/rss-$label.csv"
  npx artillery run --output "$RESULTS/raw-$label.json" "$yaml" \
    > "$RESULTS/artillery-$label.txt" 2>&1 || echo "(artillery exit $? — see txt)"
  kill "$RSS_PID" 2>/dev/null || true
  wait "$RSS_PID" 2>/dev/null || true
  python3 - "$label" "$RESULTS" <<'PY'
import json, sys, os
label, results = sys.argv[1], sys.argv[2]
p = os.path.join(results, f"raw-{label}.json")
d = json.load(open(p))["aggregate"]
c, h = d.get("counters", {}), d.get("histograms", {})
rt = h.get("http.response_time", {})
row = {
    "label": label,
    "requests": c.get("http.requests"),
    "ok_2xx": c.get("http.codes.200"),
    "err_429": c.get("http.codes.429", 0),
    "err_4xx": sum(v for k, v in c.items() if k.startswith("http.codes.4")),
    "failed": c.get("vusers.failed", 0),
    "p50": rt.get("median"), "p95": rt.get("p95"), "p99": rt.get("p99"), "max": rt.get("max"),
}
with open(os.path.join(results, "summary.jsonl"), "a") as f:
    f.write(json.dumps(row) + "\n")
print(row)
PY
}

start_engine

# pre-create channels with effectively-unlimited limits
for ch in bench-capture bench-mixed bench-replay-src bench-replay-dst; do
  curl -sf -X PUT "http://$CONTROL/api/channels/$ch" \
    -H 'content-type: application/json' \
    -d '{"rate_limit":{"burst":50000,"refill_per_s":50000}}' >/dev/null
done
echo "channels ready"

# 1-3) capture throughput (warmup → ramp → max)
run_scenario capture-warmup  bench/scenarios/capture-throughput-1-warmup.yaml
run_scenario capture-ramp    bench/scenarios/capture-throughput-2-ramp.yaml
run_scenario capture-max     bench/scenarios/capture-throughput-3-max.yaml

# 4) mixed workload (capture + history + replay-API + billing reads)
run_scenario mixed           bench/scenarios/mixed-workload.yaml

# 5) spike (100/s → 5000/s burst → recovery)
run_scenario spike           bench/scenarios/spike.yaml

# 6) soak (120s @ 400/s, 11KB bodies)
run_scenario soak            bench/scenarios/soak.yaml

# 6.5) engine ceiling probe (Go client — Artillery/Node caps out ~3k rps and
# pollutes measured latency; this measures the engine without that ceiling)
echo "== scenario: capture-ceiling (Go probe) =="
rss_sampler "$RESULTS/rss-capture-ceiling.csv"
(cd bench/harness/sub-latency && go build ./...) || { echo "harness build failed"; exit 1; }
cp bench/harness/sub-latency/sub-latency target/sub-latency
target/sub-latency -mode capture-rps -capture "http://$CAPTURE" \
  -channel bench-capture -rate 0 -duration 15s | tee "$RESULTS/capture-ceiling.json"
kill "$RSS_PID" 2>/dev/null || true

# 7) subscriber latency harness (dogfoods sdks/go)
echo "== scenario: subscriber-latency =="
rss_sampler "$RESULTS/rss-sublatency.csv"
(cd bench/harness/sub-latency && go build ./...) || { echo "harness build failed"; exit 1; }
bench/harness/sub-latency/sub-latency \
  -control "http://$CONTROL" -capture "http://$CAPTURE" \
  -channel bench-capture -subscribers 4 -rate 200 -duration 30s \
  | tee "$RESULTS/sub-latency.json"
kill "$RSS_PID" 2>/dev/null || true

echo "== done: results in $RESULTS =="
ls -la "$RESULTS"
