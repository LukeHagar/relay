//! Engine hot-path benchmarks (criterion).
//!
//! Run: `cargo bench -p relay-engine`
//! Results land in `target/criterion`; see docs/benchmarks.md for recorded baselines.

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use relay_engine::envelope::{build_envelope, CaptureInput};
use relay_engine::store::{BufferConfig, ChannelConfig, Store};

fn input(body: &str, header_count: usize) -> CaptureInput {
    let headers: Vec<(String, String)> = (0..header_count)
        .map(|i| (format!("x-header-{i}"), format!("value-{i}")))
        .chain([("content-type".into(), "application/json".into())])
        .collect();
    CaptureInput {
        channel: "bench".into(),
        host: Some("gh.acme.capture.example.com".into()),
        method: "POST".into(),
        raw_path: "/v1/webhooks/receive/a%2Fb".into(),
        raw_query: Some("b=2&a=1&sig=zz".into()),
        headers,
        body: Bytes::copy_from_slice(body.as_bytes()),
        truncated: false,
        status_sent: 200,
        remote_addr: Some("203.0.113.7:44192".into()),
    }
}

fn bench_envelope(c: &mut Criterion) {
    let mut group = c.benchmark_group("envelope_build");
    for (body_kb, hdrs) in [(1usize, 10usize), (8, 10), (8, 40), (64, 40)] {
        let body = "x".repeat(body_kb * 1024);
        group.throughput(Throughput::Bytes((body.len() + body_kb * 1024) as u64));
        group.bench_with_input(
            BenchmarkId::new("utf8", format!("{body_kb}kb/{hdrs}hdr")),
            &(body, hdrs),
            |b, (body, hdrs)| {
                let input = input(body, *hdrs);
                b.iter(|| build_envelope(&input, 42, relay_engine::envelope::now_utc()));
            },
        );
    }
    group.finish();
}

fn bench_channel_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_push");
    for max_events in [100usize, 1000, 10_000] {
        let store = Store::default();
        let ch = store
            .upsert(
                "bench",
                ChannelConfig {
                    buffer: Some(BufferConfig {
                        max_events,
                        max_age_s: 86_400,
                    }),
                    ..Default::default()
                },
            )
            .unwrap()
            .channel;
        let input = input(&"x".repeat(8 * 1024), 10);
        group.throughput(Throughput::Bytes(8 * 1024));
        group.bench_with_input(
            BenchmarkId::new("ring8kb", max_events),
            &max_events,
            |b, _| {
                let mut seq = 0u64;
                b.iter(|| {
                    seq += 1;
                    ch.push(input.clone(), relay_engine::envelope::now_utc());
                });
            },
        );
    }
    group.finish();
}

fn bench_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("history_query");
    let store = Store::default();
    let ch = store
        .upsert("bench", ChannelConfig::default())
        .unwrap()
        .channel;
    let input = input("x", 10);
    for _ in 0..1000 {
        ch.push(input.clone(), relay_engine::envelope::now_utc());
    }
    group.bench_function("full_1000", |b| b.iter(|| ch.history(None, 1000)));
    group.bench_function("cursor_500_limit_100", |b| {
        b.iter(|| ch.history(Some(500), 100))
    });
    group.bench_function("latest_1", |b| b.iter(|| ch.history(Some(999), 1)));
    group.finish();
}

fn bench_signing(c: &mut Criterion) {
    use relay_engine::signing::{sign_for, Scheme};
    let mut group = c.benchmark_group("signing");
    let body = "x".repeat(8 * 1024);
    group.throughput(Throughput::Bytes(body.len() as u64));
    for scheme in [
        Scheme::Github,
        Scheme::Stripe,
        Scheme::Slack,
        Scheme::Shopify,
        Scheme::Twilio,
    ] {
        group.bench_with_input(
            BenchmarkId::new("8kb", format!("{scheme:?}").to_lowercase()),
            &scheme,
            |b, scheme| {
                b.iter(|| {
                    sign_for(
                        *scheme,
                        "whsec_benchmark_secret",
                        body.as_bytes(),
                        1_700_000_000,
                        Some("https://gh.acme.capture.example.com/v1/hooks"),
                    )
                })
            },
        );
    }
    group.finish();
}

fn bench_interpolate(c: &mut Criterion) {
    use relay_engine::templates::{interpolate, Defaults};
    let template = serde_json::json!({
        "id": "evt_{{uuid}}",
        "created": "{{now_unix}}",
        "customer": "{{customer}}",
        "data": {
            "object": {
                "amount": "{{amount}}",
                "lines": [ {"sku": "{{sku_a}}"}, {"sku": "{{sku_b}}"} ],
                "note": "static text stays"
            }
        }
    });
    let defaults = Defaults {
        vars: [
            ("customer".to_string(), serde_json::json!("cus_bench")),
            ("amount".to_string(), serde_json::json!("4200")),
            ("sku_a".to_string(), serde_json::json!("SKU-A")),
            ("sku_b".to_string(), serde_json::json!("SKU-B")),
        ]
        .into_iter()
        .collect(),
    };
    c.bench_function("interpolate_nested_object", |b| {
        b.iter(|| interpolate(&template, &Default::default(), &defaults.vars).unwrap())
    });
}

criterion_group!(
    benches,
    bench_envelope,
    bench_channel_push,
    bench_history,
    bench_signing,
    bench_interpolate
);
criterion_main!(benches);
