use crate::Error;
use chrono::{DateTime, Utc};
use flow_lib::UserId;
use kv::{Bucket, Store};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

pub trait CacheBucket {
    type Object;
    fn name() -> &'static str;
    fn cache_time() -> Duration;
    fn can_read(obj: Self::Object, user_id: &UserId) -> bool;
}

#[derive(Clone)]
pub struct LocalStorage {
    db: kv::Store,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Jwt {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Password {
    pub password: String,
    pub encrypted_password: String,
}

fn user_key(key: &UserId) -> &[u8] {
    key.as_bytes()
}

impl LocalStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        tracing::info!("openning sled storage: {}", path.as_ref().display());
        let db = Store::new(kv::Config::new(path)).map_err(Error::local("open"))?;
        Ok(Self { db })
    }

    fn jwt_bucket(&self) -> crate::Result<Bucket<'_, &[u8], kv::Json<Jwt>>> {
        self.db
            .bucket(Some("JWTs"))
            .map_err(Error::local("open JWTs bucket"))
    }

    pub fn get_jwt(&self, user_id: &UserId) -> crate::Result<Option<Jwt>> {
        tracing::debug!("get JWTs, user_id={}", user_id);
        Ok(self
            .jwt_bucket()?
            .get(&user_key(user_id))
            .map_err(Error::local("get JWTs"))?
            .map(|j| j.0))
    }

    pub fn set_jwt(&self, user_id: &UserId, jwt: &Jwt) -> crate::Result<()> {
        tracing::debug!("set JWTs, user_id={}", user_id);
        self.jwt_bucket()?
            .set(&user_key(user_id), &kv::Json(jwt.clone()))
            .map_err(Error::local("set JWTs"))?;
        Ok(())
    }

    pub fn remove_jwt(&self, user_id: &UserId) -> crate::Result<()> {
        tracing::debug!("remove JWTs, user_id={}", user_id);
        self.jwt_bucket()?
            .remove(&user_key(user_id))
            .map_err(Error::local("remove JWTs"))?;
        Ok(())
    }

    fn password_bucket(&self) -> crate::Result<Bucket<'_, &[u8], kv::Bincode<Password>>> {
        self.db
            .bucket(Some("Passwords"))
            .map_err(Error::local("open Passwords"))
    }

    pub fn set_password(&self, user_id: &UserId, password: Password) -> crate::Result<()> {
        self.password_bucket()?
            .set(&user_key(user_id), &kv::Bincode(password))
            .map_err(Error::local("set Passwords"))?;
        Ok(())
    }

    pub fn get_or_generate_password(&self, user_id: &UserId) -> crate::Result<Password> {
        self.password_bucket()?
            .transaction::<_, kv::Error, _>(|tx| {
                if let Some(p) = tx.get(&user_key(user_id))? {
                    Ok(p.0)
                } else {
                    let password = rand_password();
                    let password = Password {
                        encrypted_password: bcrypt::hash(&password, 10).unwrap(),
                        password,
                    };
                    tx.set(&user_key(user_id), &kv::Bincode(password.clone()))?;
                    Ok(password)
                }
            })
            .map_err(Error::local("get or generate password"))
    }
}

fn rand_password() -> String {
    use rand::distributions::DistString;
    rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 24)
}
