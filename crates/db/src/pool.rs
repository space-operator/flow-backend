use crate::{
    config::{DbConfig, ProxiedDbConfig},
    connection::{
        proxied_user_conn::{self, ProxiedUserConn},
        AdminConn, UserConnection, UserConnectionTrait,
    },
    Error, LocalStorage, WasmStorage,
};
use deadpool_postgres::{ClientWrapper, Hook, HookError, Metrics, Pool, PoolConfig, SslMode};
use flow_lib::{context::get_jwt, UserId};
use futures_util::FutureExt;
use hashbrown::HashMap;
use std::time::Duration;

pub use deadpool_postgres::Object as Connection;

#[derive(Clone)]
pub enum DbPool {
    Real(RealDbPool),
    Proxied(ProxiedDbPool),
}

impl DbPool {
    pub async fn get_user_conn(
        &self,
        user_id: UserId,
    ) -> crate::Result<Box<dyn UserConnectionTrait>> {
        match self {
            DbPool::Real(pool) => Ok(Box::new(pool.get_user_conn(user_id).await?)),
            DbPool::Proxied(pool) => Ok(Box::new(pool.get_user_conn(user_id).await?)),
        }
    }

    pub fn get_local(&self) -> &LocalStorage {
        match self {
            DbPool::Real(pool) => pool.get_local(),
            DbPool::Proxied(pool) => pool.get_local(),
        }
    }
}

#[derive(Clone)]
pub struct RealDbPool {
    pg: Pool,
    wasm: WasmStorage,
    local: LocalStorage,
}

fn read_cert(path: &std::path::Path) -> crate::Result<rustls::Certificate> {
    let cert = std::fs::read(path)?;
    let mut buf = cert.as_slice();
    let items = rustls_pemfile::read_all(&mut buf)?;

    let cert = items
        .iter()
        .find_map(|i| {
            if let rustls_pemfile::Item::X509Certificate(c) = i {
                Some(rustls::Certificate(c.clone()))
            } else {
                None
            }
        })
        .ok_or(Error::NoCert)?;

    Ok(cert)
}

async fn conn_healthcheck(
    conn: &mut ClientWrapper,
    metric: &Metrics,
) -> Result<(), deadpool_postgres::HookError> {
    if metric.last_used() <= Duration::from_secs(10) {
        Ok(())
    } else {
        conn.simple_query("").await.map_err(HookError::Backend)?;
        Ok(())
    }
}

impl RealDbPool {
    pub async fn new(
        cfg: &DbConfig,
        wasm: WasmStorage,
        local: LocalStorage,
    ) -> crate::Result<Self> {
        use deadpool_postgres::{Config, Runtime};

        let pool_cfg = Config {
            user: Some(cfg.user.clone()),
            password: Some(cfg.password.clone()),
            dbname: Some(cfg.dbname.clone()),
            host: Some(cfg.host.clone()),
            port: Some(cfg.port),
            ssl_mode: Some(if cfg.ssl.enabled {
                SslMode::Require
            } else {
                SslMode::Disable
            }),
            pool: Some(PoolConfig {
                max_size: 32,
                ..Default::default()
            }),
            ..Config::default()
        };
        tracing::info!("SSL enabled: {}", cfg.ssl.enabled);

        let builder = if cfg.ssl.enabled {
            let mut roots = rustls::RootCertStore::empty();
            if let Some(path) = cfg.ssl.cert.as_ref() {
                tracing::info!("adding certificate: {}", path.display());
                let cert = read_cert(path)?;
                roots
                    .add(&cert)
                    .map_err(|e| Error::AddCert(e.to_string()))?;
            }
            let certs = rustls_native_certs::load_native_certs()
                .map_err(|e| Error::AddCert(e.to_string()))?;
            for cert in certs {
                roots
                    .add(&rustls::Certificate(cert.0))
                    .map_err(|e| Error::AddCert(e.to_string()))?;
            }
            let config = rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let tls = tokio_postgres_rustls::MakeRustlsConnect::new(config);
            pool_cfg.builder(tls).map_err(Error::CreatePool)?
        } else {
            pool_cfg
                .builder(tokio_postgres::NoTls)
                .map_err(Error::CreatePool)?
        };

        let pg = builder
            .pre_recycle(Hook::async_fn(|c, m| conn_healthcheck(c, m).boxed()))
            .runtime(Runtime::Tokio1)
            .build()
            .expect("shouldn't fail");

        // Test to see if we can connect
        let _conn = pg.get().await.map_err(Error::GetDbConnection)?;

        Ok(Self { pg, wasm, local })
    }

    pub async fn get_conn(&self) -> crate::Result<Connection> {
        let conn = tokio::time::timeout(Duration::from_secs(240), self.pg.get())
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(Error::GetDbConnection)?;
        Ok(conn)
    }

    pub async fn get_user_conn(&self, user_id: UserId) -> crate::Result<UserConnection> {
        self.get_conn()
            .await
            .map(move |conn| UserConnection::new(conn, self.wasm.clone(), user_id))
    }

    pub async fn get_admin_conn(&self) -> crate::Result<AdminConn> {
        self.get_conn().await.map(AdminConn::new)
    }

    pub fn get_local(&self) -> &LocalStorage {
        &self.local
    }
}

#[derive(Clone)]
pub struct ProxiedDbPool {
    pub config: ProxiedDbConfig,
    pub client: reqwest::Client,
    pub local: LocalStorage,
    pub services: HashMap<UserId, get_jwt::Svc>,
}

impl ProxiedDbPool {
    pub fn new(
        config: ProxiedDbConfig,
        local: LocalStorage,
        services: HashMap<UserId, get_jwt::Svc>,
    ) -> crate::Result<Self> {
        Ok(Self {
            config,
            client: reqwest::Client::new(),
            local,
            services,
        })
    }

    pub fn get_local(&self) -> &LocalStorage {
        &self.local
    }

    pub async fn get_user_conn(&self, user_id: UserId) -> crate::Result<ProxiedUserConn> {
        Ok(ProxiedUserConn {
            user_id,
            client: self.client.clone(),
            rpc_url: self.config.upstream_url.clone() + "/proxy/db_rpc",
            push_logs_url: self.config.upstream_url.clone() + "/proxy/db_push_logs",
            jwt_svc: self
                .services
                .get(&user_id)
                .ok_or(proxied_user_conn::Error::Jwt(get_jwt::Error::NotAllowed))?
                .clone(),
        })
    }
}
