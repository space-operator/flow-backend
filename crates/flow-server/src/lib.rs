use actix_web::http::header::{AUTHORIZATION, HeaderName, HeaderValue};
use anyhow::Context;
use db::config::{DbConfig, EncryptionKey, SslConfig};
use flow_lib::config::Endpoints;
use middleware::req_fn::{self, Function, ReqFn};
use rand::{Rng, thread_rng};
use serde::Deserialize;
use serde_with::serde_as;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::{path::PathBuf, rc::Rc};
use url::Url;
use user::SignatureAuth;

pub mod api;
pub mod cmd_workers;
pub mod db_worker;
pub mod error;
pub mod middleware;
pub mod user;
pub mod ws;

pub static X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");

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
                url: format!("https://{project_id}.supabase.co").parse()?,
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

fn default_db_config() -> DbConfig {
    DbConfig {
        user: "flow_runner".to_owned(),
        password: "flow_runner".to_owned(),
        dbname: "postgres".to_owned(),
        host: "localhost".to_owned(),
        port: 5432,
        ssl: SslConfig {
            use_builtin_supabase_cert: false,
            enabled: false,
            cert: None,
        },
        max_pool_size: None,
        encryption_key: Some(EncryptionKey::random()),
    }
}

#[serde_as]
#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_host")]
    pub host: String,
    #[serde(default = "Config::default_port")]
    pub port: u16,
    #[serde(default = "default_db_config")]
    pub db: DbConfig,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    pub supabase: SupabaseConfig,
    #[serde(default = "Config::default_local_storage")]
    pub local_storage: PathBuf,
    #[serde(default = "Config::default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u16,
    pub helius_api_key: Option<String>,
    pub solana: Option<SolanaConfig>,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(default = "Config::default_secret_key")]
    pub iroh_secret_key: iroh::SecretKey,

    #[serde(skip)]
    blake3_key: [u8; blake3::KEY_LEN],
}

#[derive(Deserialize, Default)]
pub struct SolanaConfig {
    pub mainnet_url: Option<Url>,
    pub devnet_url: Option<Url>,
    pub testnet_url: Option<Url>,
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
            blake3_key: rand::rngs::OsRng.r#gen(),
            solana: None,
            helius_api_key: None,
            iroh_secret_key: iroh::SecretKey::generate(&mut rand::rngs::OsRng),
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

    pub fn default_secret_key() -> iroh::SecretKey {
        iroh::SecretKey::generate(&mut thread_rng())
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

    pub async fn healthcheck(&self) -> Result<(), Vec<anyhow::Error>> {
        let mut errors = Vec::new();
        if let Some(key) = &self.helius_api_key {
            let client = RpcClient::new(format!("https://mainnet.helius-rpc.com/?api-key={key}"));
            client
                .get_version()
                .await
                .context("Helius mainnet failed")
                .map_err(|error| errors.push(error))
                .ok();
            let client = RpcClient::new(format!("https://devnet.helius-rpc.com/?api-key={key}"));
            client
                .get_version()
                .await
                .context("Helius devnet failed")
                .map_err(|error| errors.push(error))
                .ok();
        }
        if let Some(url) = self.solana.as_ref().and_then(|s| s.mainnet_url.as_ref()) {
            let client = RpcClient::new(url.to_string());
            client
                .get_version()
                .await
                .context("Solana mainnet failed")
                .map_err(|error| errors.push(error))
                .ok();
        }
        if let Some(url) = self.solana.as_ref().and_then(|s| s.devnet_url.as_ref()) {
            let client = RpcClient::new(url.to_string());
            client
                .get_version()
                .await
                .context("Solana devnet failed")
                .map_err(|error| errors.push(error))
                .ok();
        }
        if let Some(url) = self.solana.as_ref().and_then(|s| s.testnet_url.as_ref()) {
            let client = RpcClient::new(url.to_string());
            client
                .get_version()
                .await
                .context("Solana testnet failed")
                .map_err(|error| errors.push(error))
                .ok();
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn endpoints(&self) -> Endpoints {
        Endpoints {
            flow_server: match &self.db {
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

    // /// Build a middleware to validate `Authorization` header
    // /// with Supabase's JWT secret and API key.
    // pub fn all_auth(&self, pool: DbPool) -> auth::ApiAuth {
    //     match (self.supabase.jwt_key.as_ref(), pool) {
    //         (Some(key), DbPool::Real(pool)) => auth::ApiAuth::real(
    //             key.as_bytes(),
    //             self.supabase.anon_key.clone(),
    //             pool,
    //             self.signature_auth(),
    //         ),
    //         (None, DbPool::Real(pool)) => {
    //             // TODO: print error
    //             auth::ApiAuth::real(
    //                 &[],
    //                 self.supabase.anon_key.clone(),
    //                 pool,
    //                 self.signature_auth(),
    //             )
    //         }
    //     }
    // }

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

    /// Build a middleware to validate `Authorization` header
    /// with Supabase's service-role key.
    pub fn service_key(&self) -> Option<ReqFn<Rc<dyn Function>>> {
        let key = self.supabase.service_key.clone()?;
        Some(req_fn::rc_fn_ref(move |r| {
            match r.headers().get(&AUTHORIZATION) {
                Some(v) => {
                    if v.as_bytes().strip_prefix(b"Bearer ") == Some(key.as_bytes()) {
                        Ok(())
                    } else {
                        Err(error::ApiKey)
                    }
                }
                _ => Err(error::ApiKey),
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use flow::FlowGraph;
    use flow_lib::{
        FlowConfig, command::CommandDescription, config::client::ClientConfig,
        flow_run_events::event_channel,
    };
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

    #[actix::test]
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
        dbg!(&res.output);
        dbg!(&res.node_errors);
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

    #[actix::test]
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

    #[actix::test]
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

    #[actix::test]
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

    #[actix::test]
    async fn test_flow_input() {
        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!("../../../test_files/HTTP Request.json");
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
        let mut m = BTreeSet::new();
        let mut dup = false;
        for CommandDescription { matcher, .. } in inventory::iter::<CommandDescription>() {
            let name = match matcher.name.clone() {
                flow_lib::command::MatchName::Exact(cow) => cow,
                flow_lib::command::MatchName::Regex(cow) => cow,
            };
            if !m.insert(name.clone()) {
                println!("Dupicated: {name}");
                dup = true;
            }
        }
        assert!(!dup);
    }
}
