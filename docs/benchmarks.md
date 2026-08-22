# Engine benchmarks

Criterion benches for the engine hot paths: `cargo bench -p relay-engine`
(source: `crates/engine/benches/hot_paths.rs`). Results land in `target/criterion`;
this file records the baseline from each release-relevant run.

## Baseline — 2026-08-21, bench profile, AMD Ryzen 9 7950X3D (Linux)

| Benchmark | low | median | high |
|---|---|---|---|
| `channel_push/ring8kb/100` | 1.34 µs | **1.36 µs** | 1.39 µs |
| `channel_push/ring8kb/1000` | 1.46 µs | **1.50 µs** | 1.54 µs |
| `channel_push/ring8kb/10000` | 2.35 µs | **2.41 µs** | 2.46 µs |
| `envelope_build/utf8/1kb/10hdr` | 555 ns | **563 ns** | 571 ns |
| `envelope_build/utf8/64kb/40hdr` | 3.64 µs | **3.70 µs** | 3.75 µs |
| `envelope_build/utf8/8kb/10hdr` | 763 ns | **779 ns** | 797 ns |
| `envelope_build/utf8/8kb/40hdr` | 1.95 µs | **2.01 µs** | 2.07 µs |
| `history_query/cursor_500_limit_100` | 1.04 µs | **1.06 µs** | 1.07 µs |
| `history_query/full_1000` | 5.57 µs | **5.60 µs** | 5.63 µs |
| `history_query/latest_1` | 716 ns | **739 ns** | 767 ns |
| `interpolate_nested_object` | 838 ns | **853 ns** | 870 ns |
| `signing/8kb/github` | 3.60 µs | **3.61 µs** | 3.63 µs |
| `signing/8kb/shopify` | 3.53 µs | **3.55 µs** | 3.57 µs |
| `signing/8kb/slack` | 3.75 µs | **3.77 µs** | 3.78 µs |
| `signing/8kb/stripe` | 3.70 µs | **3.72 µs** | 3.74 µs |
| `signing/8kb/twilio` | 17.34 µs | **17.63 µs** | 17.94 µs |

Notes:

- `channel_push` includes envelope build + ring-buffer insert under the channel lock;
  eviction shows up as `max_events` grows but stays flat per event.
- `signing` is ~3.5–4 µs for an 8 KiB body across schemes; Twilio costs more because
  its canonical string includes the URL plus sorted form params.
- `history_query` scans a VecDeque — draining 1000 events ≈ 5.7 µs.
