use db::{LocalStorage, WasmStorage, pool::DbPool};
use url::Url;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let config = toml::from_str::<db::config::DbConfig>(
        &std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap(),
    )
    .unwrap();
    DbPool::new(
        &config,
        WasmStorage::new(Url::parse("http://localhost").unwrap(), "", "").unwrap(),
        LocalStorage::new("/tmp/local_storage").unwrap(),
    )
    .await
    .unwrap();
}
