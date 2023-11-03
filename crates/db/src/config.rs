use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct DbConfig {
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub host: String,
    pub port: u16,
    pub ssl: Option<SslConfig>,
}

#[derive(Deserialize, Clone)]
pub struct SslConfig {
    pub cert: std::path::PathBuf,
}

impl ToString for DbConfig {
    fn to_string(&self) -> String {
        format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, self.dbname,
        )
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
            ssl: None,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ProxiedDbConfig {
    pub upstream_url: String,
    pub api_keys: Vec<String>,
}
