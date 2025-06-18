use command_rpc::command_side::command_server::Config;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut data = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let config: Config = simd_json::from_slice(&mut data).unwrap();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(command_rpc::command_side::command_server::serve(config))
        .await
        .unwrap();
}
