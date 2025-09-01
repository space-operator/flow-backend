use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use flow::FlowGraph;
use flow_lib::{FlowConfig, config::client::ClientConfig};
use tokio::task::LocalSet;

use cmds_std as _;

#[derive(serde::Deserialize)]
struct TestFile {
    flow: ClientConfig,
}

async fn new(config: &FlowConfig) {
    FlowGraph::from_cfg(config.clone(), <_>::default(), None)
        .await
        .unwrap();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    const JSON: &str = include_str!("../../../test_files/const_form_data.json");
    let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(JSON).unwrap().flow);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = LocalSet::new();
    c.bench_function("new_const_form_data", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _i in 0..iters {
                std::hint::black_box(local.block_on(&rt, new(&flow_config)));
            }
            start.elapsed()
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
