use criterion::{criterion_group, criterion_main, Criterion};
use srpc::{GetBaseUrl, RegisterJsonService};
use std::convert::Infallible;
use tungstenite::Message;

fn make_ws_url(url: &str) -> String {
    let url = url
        .strip_prefix("http")
        .unwrap()
        .strip_suffix("call")
        .unwrap();
    format!("ws{}ws", url)
}

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

    c.bench_function("srpc_http1", |b| {
        let client = reqwest::blocking::ClientBuilder::new().build().unwrap();
        let body = r#"{"envelope":"","svc_name":"add","svc_id":"","input":[1, 2]}"#;
        let req = client
            .post(&url)
            .header("content-type", "application/json")
            .body(body);
        b.iter(|| {
            let body = req.try_clone().unwrap().send().unwrap().text().unwrap();
            assert_eq!(body, r#"{"envelope":"","success":true,"data":3}"#,);
        });
    });

    let ws_url = make_ws_url(&url);

    c.bench_function("srpc_ws", |b| {
        let body = r#"{"envelope":"","svc_name":"add","svc_id":"","input":[1, 2]}"#;
        let (mut conn, _) = tungstenite::connect(&ws_url).unwrap();
        b.iter(|| {
            conn.send(Message::Text(body.to_owned())).unwrap();
            let Ok(Message::Text(body)) = conn.read() else {
                panic!();
            };
            assert_eq!(body, r#"{"envelope":"","success":true,"data":3}"#);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
