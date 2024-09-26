use std::convert::Infallible;

use criterion::{criterion_group, criterion_main, Criterion};
use srpc::{GetBaseUrl, RegisterJsonService};

pub fn criterion_benchmark(c: &mut Criterion) {
    let (url_tx, url_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(|| {
        actix::run(async move {
            let addr = srpc::Server::start_http_server().unwrap();
            addr.send(RegisterJsonService::new(
                "add".to_owned(),
                "".to_owned(),
                tower::service_fn(|(a, b): (i64, i64)| async move { Ok::<_, Infallible>(a + b) }),
            ))
            .await
            .unwrap();
            let url = addr
                .send(GetBaseUrl)
                .await
                .unwrap()
                .unwrap()
                .join("/call")
                .unwrap()
                .to_string();
            url_tx.send(url).unwrap();
            std::future::pending::<()>().await;
        })
        .unwrap();
    });

    let url = url_rx.blocking_recv().unwrap();
    c.bench_function("srpc_http", |b| {
        let client = reqwest::Client::new();
        let body = r#"{"envelope":"","svc_name":"add","svc_id":"","input":[1, 2]}"#;
        let req = client
            .post(&url)
            .header("content-type", "application/json")
            .body(body);
        b.to_async(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        )
        .iter(|| async {
            let body = req
                .try_clone()
                .unwrap()
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            assert_eq!(body, r#"{"envelope":"","success":true,"data":3}"#,);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
