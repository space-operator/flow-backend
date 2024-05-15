use actix_web::http::header::{HeaderName, HeaderValue};
use db::{
    config::{DbConfig, ProxiedDbConfig},
    pool::DbPool,
};
use either::Either;
use flow_lib::config::Endpoints;
use middleware::{
    auth,
    req_fn::{self, Function, ReqFn},
};
use serde::Deserialize;
use std::{path::PathBuf, rc::Rc};
use url::Url;
use user::SignatureAuth;

pub mod api;
pub mod db_worker;
pub mod error;
pub mod flow_logs;
pub mod middleware;
pub mod user;
pub mod ws;

pub fn match_wildcard(pat: &str, origin: &HeaderValue) -> bool {
    let Ok(mut origin_str) = origin.to_str() else {
        return false;
    };

    let mut segments = pat.split('*');

    let Some(first) = segments.next() else {
        return false;
    };
    origin_str = match origin_str.strip_prefix(first) {
        Some(s) => s,
        None => return false,
    };

    for s in segments {
        if s.is_empty() {
            continue;
        }
        match origin_str.find(s) {
            Some(pos) => {
                let wildcard = &origin_str[..pos];
                if !wildcard.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return false;
                }
                origin_str = &origin_str[pos..];
            }
            None => {
                return false;
            }
        }
    }

    true
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EndpointConfigUnchecked {
    ProjectId { project_id: String },
    Endpoint { endpoint: Url },
}

#[derive(Deserialize, Clone)]
#[serde(try_from = "EndpointConfigUnchecked")]
pub struct EndpointConfig {
    url: Url,
}

impl TryFrom<EndpointConfigUnchecked> for EndpointConfig {
    type Error = url::ParseError;
    fn try_from(value: EndpointConfigUnchecked) -> Result<Self, Self::Error> {
        Ok(match value {
            EndpointConfigUnchecked::Endpoint { endpoint } => Self { url: endpoint },
            EndpointConfigUnchecked::ProjectId { project_id } => Self {
                url: format!("https://{}.supabase.co", project_id).parse()?,
            },
        })
    }
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            // default location of Supabase CLI local development
            url: "http://localhost:54321".parse().unwrap(),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct SupabaseConfig {
    #[serde(flatten)]
    pub endpoint: EndpointConfig,
    pub jwt_key: Option<String>,
    pub anon_key: String,
    pub service_key: Option<String>,
    #[serde(default = "SupabaseConfig::default_bucket")]
    pub wasm_bucket: String,
    #[serde(default = "SupabaseConfig::default_open_whitelists")]
    pub open_whitelists: bool,
}

impl SupabaseConfig {
    pub fn default_bucket() -> String {
        "node-files".to_owned()
    }

    pub fn default_open_whitelists() -> bool {
        false
    }

    pub fn get_endpoint(&self) -> Url {
        self.endpoint.url.clone()
    }
}

impl Default for SupabaseConfig {
    fn default() -> Self {
        Self {
            endpoint: <_>::default(),
            jwt_key: None,
            anon_key: String::new(),
            service_key: None,
            wasm_bucket: Self::default_bucket(),
            open_whitelists: Self::default_open_whitelists(),
        }
    }
}

fn default_db_config() -> Either<DbConfig, ProxiedDbConfig> {
    either::Right(ProxiedDbConfig {
        upstream_url: "https://dev-api.spaceoperator.com".parse().unwrap(),
        api_keys: Vec::new(),
    })
}

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default = "Config::default_host")]
    pub host: String,
    #[serde(default = "Config::default_port")]
    pub port: u16,
    #[serde(default = "default_db_config", with = "either::serde_untagged")]
    pub db: Either<DbConfig, ProxiedDbConfig>,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    pub supabase: SupabaseConfig,
    #[serde(default = "Config::default_local_storage")]
    pub local_storage: PathBuf,
    #[serde(default = "Config::default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u16,
    pub helius_api_key: Option<String>,

    #[serde(skip)]
    blake3_key: [u8; blake3::KEY_LEN],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            db: default_db_config(),
            cors_origins: Vec::new(),
            supabase: SupabaseConfig::default(),
            local_storage: Self::default_local_storage(),
            shutdown_timeout_secs: Self::default_shutdown_timeout_secs(),
            blake3_key: rand::random(),
            helius_api_key: None,
        }
    }
}

impl Config {
    pub fn default_host() -> String {
        "127.0.0.1".to_owned()
    }

    pub const fn default_port() -> u16 {
        8080
    }

    pub fn default_local_storage() -> PathBuf {
        PathBuf::from("./local_storage")
    }

    pub const fn default_shutdown_timeout_secs() -> u16 {
        1
    }

    pub fn get_config() -> Self {
        match std::env::args().nth(1) {
            Some(s) => if s == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .map_err(|error| {
                        tracing::error!("Error reading STDIN: {}", error);
                    })
                    .map(move |_| buf)
            } else {
                std::fs::read_to_string(s).map_err(|error| {
                    tracing::error!("Error reading config: {}", error);
                })
            }
            .and_then(|s| {
                toml::from_str(&s).map_err(|error| {
                    tracing::error!("Error parsing config: {}", error);
                })
            })
            .map_err(|_| {
                tracing::warn!("Invalid config file, using default");
            })
            .unwrap_or_default(),
            None => {
                tracing::info!("No config specified, using default");
                Config::default()
            }
        }
    }

    pub fn endpoints(&self) -> Endpoints {
        Endpoints {
            flow_server: match &self.db {
                Either::Right(cfg) => cfg.upstream_url.to_string(),
                _ => format!("http://localhost:{}", self.port),
            },
            supabase: self.supabase_endpoint(),
            supabase_anon_key: self.supabase.anon_key.clone(),
        }
    }

    fn supabase_endpoint(&self) -> String {
        // TODO: strip / to avoid breaking changes
        let mut s = self.supabase.get_endpoint().to_string();
        if s.ends_with('/') {
            s.pop();
        }
        s
    }

    /// Build a CORS middleware.
    pub fn cors(&self) -> actix_cors::Cors {
        let mut cors = actix_cors::Cors::default()
            .allow_any_header()
            .allow_any_method()
            .supports_credentials();
        if self.cors_origins.iter().any(|v| v == "*") {
            cors = cors.allow_any_origin();
        } else {
            for origin in &self.cors_origins {
                if origin.contains('*') {
                    let pattern = origin.clone();
                    cors =
                        cors.allowed_origin_fn(move |origin, _| match_wildcard(&pattern, origin));
                } else {
                    cors = cors.allowed_origin(origin);
                }
            }
        }
        cors
    }

    pub fn signature_auth(&self) -> SignatureAuth {
        SignatureAuth::new(self.blake3_key)
    }

    /// Build a middleware to validate `Authorization` header
    /// with Supabase's JWT secret and API key.
    pub fn all_auth(&self, pool: DbPool) -> auth::ApiAuth {
        match (self.supabase.jwt_key.as_ref(), pool) {
            (Some(key), DbPool::Real(pool)) => auth::ApiAuth::real(
                key.as_bytes(),
                self.supabase.anon_key.clone(),
                pool,
                self.signature_auth(),
            ),
            (None, DbPool::Real(pool)) => {
                // TODO: print error
                auth::ApiAuth::real(
                    &[],
                    self.supabase.anon_key.clone(),
                    pool,
                    self.signature_auth(),
                )
            }
            (_, DbPool::Proxied(pool)) => auth::ApiAuth::proxied(pool, self.signature_auth()),
        }
    }

    /// Build a middleware to validate `apikey` header
    /// with Supabase's anon key.
    pub fn anon_key(&self) -> ReqFn<Rc<dyn Function>> {
        let key = self.supabase.anon_key.clone();
        let name = HeaderName::from_static("apikey");
        req_fn::rc_fn_ref(move |r| match r.headers().get(&name) {
            Some(v) if key.as_bytes() == v => Ok(()),
            _ => Err(error::ApiKey),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::{flow_run_events::event_channel, FlowGraph};
    use flow_lib::{command::CommandDescription, config::client::ClientConfig, FlowConfig};
    use value::Value;

    use cmds_solana as _;
    use cmds_std as _;

    #[derive(Deserialize)]
    struct TestFile {
        flow: ClientConfig,
    }

    #[test]
    fn cors_wildcard() {
        assert!(match_wildcard(
            "https://flow-git-*-space-operator.vercel.app",
            &HeaderValue::from_static("https://flow-git-master-space-operator.vercel.app"),
        ));
        assert!(match_wildcard(
            "https://flow-git-*-space-operator.vercel.app",
            &HeaderValue::from_static("https://flow-git-flows-space-operator.vercel.app"),
        ));
        assert!(match_wildcard(
            "https://flow-*-space-operator.vercel.app",
            &HeaderValue::from_static("https://flow-qv9tx6vxs-space-operator.vercel.app"),
        ));
        assert!(!match_wildcard(
            "https://flow-*-space-operator.vercel.app",
            &HeaderValue::from_static("https://flow-qv9tx6vxs-fake-space-operator.vercel.app"),
        ));
    }

    #[tokio::test]
    async fn test_generate_keypair() {
        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!("../../../test_files/generate_keypair.json");
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        assert_eq!(
            res.output["key"],
            Value::new_keypair_bs58("3LUpzbebV5SCftt8CPmicbKxNtQhtJegEz4n8s6LBf3b1s4yfjLapgJhbMERhP73xLmWEP2XJ2Rz7Y3TFiYgTpXv").unwrap(),
        );
        // balance might change on solana devnet, so don't assert value here
        assert!(res.output.contains_key("balance"));
        assert!(res.node_errors.is_empty());
        assert!(res.not_run.is_empty());
        println!(
            "output: {}",
            serde_json::to_string_pretty(&res.output).unwrap()
        );
    }

    #[tokio::test]
    async fn test_const_form_data() {
        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!("../../../test_files/const_form_data.json");
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        assert!(res.node_errors.is_empty());
        // TODO: check output values
    }

    #[tokio::test]
    async fn test_foreach() {
        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!("../../../test_files/foreach.json");
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        assert_eq!(res.output["keypairs"], Value::Array([
            Value::new_keypair_bs58("3LUpzbebV5SCftt8CPmicbKxNtQhtJegEz4n8s6LBf3b1s4yfjLapgJhbMERhP73xLmWEP2XJ2Rz7Y3TFiYgTpXv").unwrap(),
            Value::new_keypair_bs58("5WmnrZDv6oM4tkN5SfSTf5MGyPLPV4HjHGQZN4JiBDCxkcetz5LTYYhRwNxKXY5BCWBaVYALZ2GkpBpU5uRr2jMa").unwrap(),
            Value::new_keypair_bs58("XunqA3LMMvpjD1JH9HMp2eSmvEaSoTdGhnNrseoFW9rMsSRhVefZYcTRDdfgVxoyJJvLwF2gzV4zyYMGiAoJaSS").unwrap(),
        ].to_vec()));
    }

    #[tokio::test]
    async fn test_file_upload() {
        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!("../../../test_files/file_upload.json");
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        dbg!(res);
    }

    #[test]
    fn test_name_unique() {
        let mut m = std::collections::HashSet::new();
        let mut dup = false;
        for CommandDescription { name, .. } in inventory::iter::<CommandDescription>() {
            if !m.insert(name) {
                println!("Dupicated: {}", name);
                dup = true;
            }
        }
        assert!(!dup);
    }
}
