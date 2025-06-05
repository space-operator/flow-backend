use cmds_std::postgrest::builder_select;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use flow_lib::{
    Value,
    context::CommandContext,
    value::{self, array, map},
};
use reqwest::header::{HeaderMap, HeaderValue};

fn build_header() -> HeaderMap {
    let mut map = HeaderMap::new();
    map.insert("accept", HeaderValue::from_str("application/json").unwrap());
    map
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let json = serde_json::from_str::<serde_json::Value>(
        r#"
{
  "url": "https://base.spaceoperator.com/rest/v1/table",
  "body": null,
  "is_rpc": false,
  "method": "GET",
  "schema": null,
  "headers": [
    [
      "accept",
      "application/json"
    ]
  ],
  "queries": []
}"#,
    )
    .unwrap();
    let params = map! {
        "query" => json,
        "columns" => "*",
    };
    let cmd = builder_select::build().unwrap();
    let ctx = CommandContext::test_context();
    c.bench_function("run_command", |b| {
        b.iter(|| {
            let fut = cmd.run(black_box(ctx.clone()), black_box(params.clone()));
            futures_executor::block_on(fut)
        })
    });
    c.bench_function("deserialize", |b| {
        b.iter(|| value::from_map::<builder_select::Input>(black_box(params.clone())).unwrap())
    });
    let query = value::from_map::<builder_select::Input>(params)
        .unwrap()
        .query;
    c.bench_function("serialize", |b| {
        b.iter(|| {
            value::to_map(&black_box(builder_select::Output {
                query: query.clone(),
            }))
            .unwrap()
        })
    });
    c.bench_function("build_header", |b| b.iter(build_header));
    let value = Value::Array(array![array!["accept", "application/json"]]);
    c.bench_function("deser_vec_tuple", |b| {
        b.iter(|| value::from_value::<Vec<(String, String)>>(black_box(value.clone())).unwrap())
    });
    c.bench_function("new_reqwest_client", |b| b.iter(reqwest::Client::new));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
