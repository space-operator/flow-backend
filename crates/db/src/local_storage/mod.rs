use crate::Error;
use chrono::{DateTime, Utc};
use flow_lib::UserId;
use kv::{Bucket, Store};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{path::Path, time::Duration};

pub trait CacheBucket {
    type Key: ?Sized;
    type EncodedKey: for<'a> kv::Key<'a>;

    type Object: Serialize + DeserializeOwned;
    fn name() -> &'static str;
    fn encode_key(key: &Self::Key) -> Self::EncodedKey;
    fn cache_time() -> Duration;
    fn can_read(obj: &Self::Object, user_id: &UserId) -> bool;
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

#[derive(Serialize, Deserialize)]
struct CacheValue<V> {
    expires_at: i64,
    value: V,
}

impl LocalStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        tracing::info!("openning sled storage: {}", path.as_ref().display());
        let db = Store::new(kv::Config::new(path)).map_err(Error::local("open"))?;
        Ok(Self { db })
    }

    pub fn set_cache<C>(&self, key: &C::Key, value: C::Object) -> crate::Result<()>
    where
        C: CacheBucket,
    {
        let bucket = C::name();
        tracing::debug!("set_cache {}", bucket);
        self.db
            .bucket::<C::EncodedKey, kv::Json<CacheValue<C::Object>>>(Some(bucket))
            .map_err(Error::local("open cache bucket"))?
            .set(
                &C::encode_key(key),
                &kv::Json(CacheValue {
                    expires_at: Utc::now().timestamp() + C::cache_time().as_secs() as i64,
                    value,
                }),
            )
            .map_err(Error::local(bucket))?;
        Ok(())
    }

    pub fn get_cache<C>(&self, user_id: &UserId, key: &C::Key) -> Option<C::Object>
    where
        C: CacheBucket,
    {
        let bucket = C::name();
        tracing::trace!("get_cache {}", bucket);
        let result = self
            .db
            .bucket::<_, kv::Json<CacheValue<C::Object>>>(Some(bucket))
            .inspect_err(|error| {
                tracing::error!("get_cache error: {}", error);
            })
            .ok()?
            .transaction::<_, kv::Error, _>(|tx| {
                let now = Utc::now().timestamp();
                let key = C::encode_key(key);
                if let Some(obj) = tx.get(&key)? {
                    if obj.0.expires_at <= now {
                        tx.remove(&key)?;
                        Ok(None)
                    } else if C::can_read(&obj.0.value, user_id) {
                        tracing::debug!("cache hit {}", bucket);
                        Ok(Some(obj.0.value))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            });
        match result {
            Ok(result) => result,
            Err(error) => {
                tracing::error!("get_cache error: {}", error);
                None
            }
        }
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
        tracing::debug!("get password {}", user_id);
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
