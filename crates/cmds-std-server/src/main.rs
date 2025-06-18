#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(command_rpc::command_side::command_server::serve(todo!()))
        .await
        .unwrap();
}
