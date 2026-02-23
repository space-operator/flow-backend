use crate::{
    Error, LocalStorage, WasmStorage,
    config::{DbConfig, Encrypted, EncryptionKey},
    connection::{AdminConn, UserConnection, UserConnectionTrait},
};
use deadpool_postgres::{ClientWrapper, Hook, HookError, Metrics, Pool, PoolConfig, SslMode};
use flow_lib::{UserId, solana::Keypair};
use futures_util::FutureExt;
use std::time::{Duration, Instant};

pub use deadpool_postgres::Object as Connection;

static BUILTIN_SUPABASE_CERT: &str = include_str!("../../../certs/supabase-prod-ca-2021.crt");

#[derive(Clone)]
pub struct DbPool {
    encryption_key: Option<EncryptionKey>,
    pg: Pool,
    wasm: WasmStorage,
    local: LocalStorage,
}

fn read_cert(path: &std::path::Path) -> crate::Result<rustls::Certificate> {
    let cert = std::fs::read(path)?;
    parse_cert(&cert)
}

fn parse_cert(mut cert: &[u8]) -> crate::Result<rustls::Certificate> {
    let items = rustls_pemfile::read_all(&mut cert)?;

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

impl DbPool {
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
            pool: cfg.max_pool_size.map(|size| PoolConfig {
                max_size: size,
                ..Default::default()
            }),
            ..Config::default()
        };
        tracing::info!("SSL enabled: {}", cfg.ssl.enabled);
        let encryption_key = cfg.encryption_key.clone();

        let builder = if cfg.ssl.enabled {
            let mut roots = rustls::RootCertStore::empty();
            if let Some(path) = cfg.ssl.cert.as_ref() {
                tracing::info!("adding certificate: {}", path.display());
                let cert = read_cert(path)?;
                roots
                    .add(&cert)
                    .map_err(|e| Error::AddCert(e.to_string()))?;
            }
            if cfg.ssl.use_builtin_supabase_cert {
                tracing::info!("adding certificate: supabase-prod-ca-2021.crt");
                roots
                    .add(&parse_cert(BUILTIN_SUPABASE_CERT.as_bytes())?)
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
        let conn = pg.get().await.map_err(Error::GetDbConnection)?;
        match ping(&conn).await {
            Ok((mean, std)) => {
                tracing::info!("connection ping: {:.2}±{:.2}ms", mean, std);
            }
            Err(error) => {
                tracing::error!("{}", error);
            }
        }

        {
            let pg = pg.clone();
            let max_age = Duration::from_secs(30);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(45)).await;
                    if pg.is_closed() {
                        break;
                    }

                    // close connections if they are unused for 30 secs
                    pg.retain(|_, metrics| metrics.last_used() < max_age);
                }
            });
        };

        Ok(Self {
            pg,
            wasm,
            local,
            encryption_key,
        })
    }

    pub(crate) fn encryption_key(&self) -> crate::Result<&EncryptionKey> {
        self.encryption_key
            .as_ref()
            .ok_or(crate::Error::NoEncryptionKey)
    }

    pub fn encrypt_keypair(&self, keypair: &Keypair) -> crate::Result<Encrypted> {
        Ok(self.encryption_key()?.encrypt_keypair(keypair))
    }

    pub async fn get_conn(&self) -> crate::Result<Connection> {
        // let conn = tokio::time::timeout(Duration::from_secs(240), self.pg.get())
        //     .await
        //     .map_err(|_| Error::Timeout)?
        //     .map_err(Error::GetDbConnection)?;
        // Ok(conn)
        self.pg.get().await.map_err(Error::GetDbConnection)
    }

    pub async fn get_user_conn(
        &self,
        user_id: UserId,
    ) -> crate::Result<Box<dyn UserConnectionTrait>> {
        Ok(Box::new(UserConnection::new(
            self.clone(),
            self.wasm.clone(),
            user_id,
            self.local.clone(),
        )))
    }

    pub async fn get_admin_conn(&self) -> crate::Result<AdminConn> {
        Ok(AdminConn::new(self.clone(), self.local.clone()))
    }

    pub fn get_local(&self) -> &LocalStorage {
        &self.local
    }

    /// Look up a SWIG session secret key from the swig_wallet_sessions table.
    ///
    /// Returns the base58-encoded secret key if a matching active session exists
    /// with a stored secret_key_encrypted value.
    pub async fn get_swig_session_secret_key(
        &self,
        swig_wallet_id: &str,
        session_pubkey: &str,
    ) -> crate::Result<Option<String>> {
        use crate::connection::DbClient;

        let conn = self.get_conn().await?;
        let swig_wallet_uuid = uuid::Uuid::parse_str(swig_wallet_id)
            .map_err(|e| crate::Error::LogicError(e.into()))?;
        let row = conn
            .do_query_opt(
                "SELECT secret_key_encrypted
                 FROM swig_wallet_sessions
                 WHERE swig_wallet_id = $1
                   AND session_pubkey = $2
                   AND status = 'active'
                   AND (expires_at IS NULL OR expires_at > NOW())
                 LIMIT 1",
                &[&swig_wallet_uuid, &session_pubkey],
            )
            .await
            .map_err(crate::Error::exec("get_swig_session_secret_key"))?;
        Ok(row.and_then(|r| r.get::<_, Option<String>>(0)))
    }
}

async fn ping(conn: &Connection) -> crate::Result<(f64, f64)> {
    let stmt = conn
        .prepare_cached("SELECT gen_random_uuid()")
        .await
        .map_err(Error::exec("prepare"))?;

    let mut time = Vec::new();

    for _ in 0..10 {
        let now = Instant::now();
        conn.query_one(&stmt, &[])
            .await
            .map_err(Error::exec("select"))?;
        let elapsed = now.elapsed();
        time.push(elapsed.as_secs_f64() * 1000.0);
    }

    let mean = time.iter().sum::<f64>() / time.len() as f64;
    let std =
        (time.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / time.len() as f64).sqrt();

    Ok((mean, std))
}
