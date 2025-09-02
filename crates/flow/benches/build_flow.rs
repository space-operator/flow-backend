use std::time::Instant;

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use flow::{FlowGraph, flow_registry::FlowRegistry};
use flow_lib::{FlowConfig, config::client::ClientConfig};
use tokio::task::LocalSet;

use cmds_std as _;

#[derive(serde::Deserialize)]
struct TestFile {
    flow: ClientConfig,
}

async fn new(config: &FlowConfig, registry: &FlowRegistry) {
    FlowGraph::from_cfg(config.clone(), registry.clone(), None)
        .await
        .unwrap();
}

fn bench_file(
    b: &mut Bencher,
    json: &str,
    rt: &tokio::runtime::Runtime,
    local: &LocalSet,
    registry: &FlowRegistry,
) {
    let flow = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);

    b.iter_custom(|iters| {
        let start = Instant::now();
        for _i in 0..iters {
            std::hint::black_box(local.block_on(&rt, new(&flow, &registry)));
        }
        start.elapsed()
    });
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = LocalSet::new();
    let registry = FlowRegistry::default();

    c.bench_function("new_const_form_data_flow", |b| {
        bench_file(
            b,
            include_str!("../../../test_files/const_form_data.json"),
            &rt,
            &local,
            &registry,
        )
    });

    c.bench_function("new_http_request_flow", |b| {
        bench_file(
            b,
            include_str!("../../../test_files/HTTP Request.json"),
            &rt,
            &local,
            &registry,
        )
    });

    rt.shutdown_background();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
