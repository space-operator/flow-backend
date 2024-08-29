use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize, Clone)]
pub struct DbConfig {
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub ssl: SslConfig,
    pub max_pool_size: Option<usize>,
}

#[derive(Deserialize, Clone, Default)]
pub struct SslConfig {
    pub enabled: bool,
    pub cert: Option<std::path::PathBuf>,
}

impl Display for DbConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, self.dbname,
        ))
    }
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            user: "postgres".into(),
            password: "spacepass".into(),
            dbname: "space-operator-db".into(),
            host: "127.0.0.1".into(),
            port: 7979,
            ssl: <_>::default(),
            max_pool_size: None,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ProxiedDbConfig {
    pub upstream_url: String,
    pub api_keys: Vec<String>,
}
