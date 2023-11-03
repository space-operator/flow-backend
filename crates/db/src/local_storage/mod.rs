use crate::Error;
use chrono::{DateTime, Utc};
use flow_lib::UserId;
use kv::Store;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

impl LocalStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        tracing::info!("openning sled storage: {}", path.as_ref().display());
        let db = Store::new(kv::Config::new(path)).map_err(Error::local("open"))?;
        Ok(Self { db })
    }

    pub fn get_jwt(&self, user_id: &UserId) -> crate::Result<Option<Jwt>> {
        tracing::debug!("get JWTs, user_id={}", user_id);
        Ok(self
            .db
            .bucket::<&[u8], kv::Json<Jwt>>(Some("JWTs"))
            .map_err(Error::local("open JWTs"))?
            .get(&&user_id.as_bytes()[..])
            .map_err(Error::local("get JWTs"))?
            .map(|j| j.0))
    }

    pub fn set_jwt(&self, user_id: &UserId, jwt: &Jwt) -> crate::Result<()> {
        tracing::debug!("set JWTs, user_id={}", user_id);
        self.db
            .bucket::<&[u8], String>(Some("JWTs"))
            .map_err(Error::local("open JWTs"))?
            .set(
                &&user_id.as_bytes()[..],
                &serde_json::to_string(jwt).unwrap(),
            )
            .map_err(Error::local("set JWTs"))?;
        Ok(())
    }

    pub fn remove_jwt(&self, user_id: &UserId) -> crate::Result<()> {
        tracing::debug!("remove JWTs, user_id={}", user_id);
        self.db
            .bucket::<&[u8], kv::Raw>(Some("JWTs"))
            .map_err(Error::local("open JWTs"))?
            .remove(&&user_id.as_bytes()[..])
            .map_err(Error::local("remove JWTs"))?;
        Ok(())
    }
}
