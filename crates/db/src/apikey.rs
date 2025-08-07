use crate::{
    Error,
    connection::{AdminConn, DbClient, UserConnection},
};
use chrono::NaiveDateTime;
use flow_lib::UserId;
use serde::Serialize;
use thiserror::Error as ThisError;
use tokio_postgres::error::SqlState;

#[derive(Clone, Serialize)]
pub struct KeyInfo {
    pub name: String,
    pub user_id: UserId,
    pub created_at: NaiveDateTime,
}

impl KeyInfo {
    pub fn new(name: &str, user_id: UserId) -> Self {
        Self {
            name: name.trim().to_owned(),
            user_id,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct APIKey {
    pub key_hash: String,
    pub trimmed_key: String,
    #[serde(flatten)]
    pub info: KeyInfo,
}

/// A BLAKE3 key
pub const KEY_PREFIX: &str = "b3-";

impl APIKey {
    pub fn generate<R: rand::Rng + rand::CryptoRng>(rng: &mut R, info: KeyInfo) -> (Self, String) {
        let mut key = KEY_PREFIX.to_owned();
        key.push_str(&base64::encode_config(
            rng.r#gen::<[u8; 32]>(),
            base64::URL_SAFE_NO_PAD,
        ));
        let trimmed_key = "*****".to_owned() + &key[key.len() - 5..];
        let key_hash =
            base64::encode_config(blake3::hash(key.as_bytes()).as_bytes(), base64::URL_SAFE);
        (
            Self {
                key_hash,
                trimmed_key,
                info,
            },
            key,
        )
    }
}

#[derive(ThisError, Debug)]
#[error("name-conflict")]
pub struct NameConflict;

fn convert_error(error: Error) -> Error<NameConflict> {
    match error {
        Error::PolarsError(e) => Error::PolarsError(e),
        Error::Unauthorized => Error::Unauthorized,
        Error::SpawnError(e) => Error::SpawnError(e),
        Error::EncryptionError => Error::EncryptionError,
        Error::Timeout => Error::Timeout,
        Error::NoEncryptionKey => Error::NoEncryptionKey,
        Error::NotSupported => Error::NotSupported,
        Error::LogicError(_) => unreachable!("get_conn should not return this variant"),
        Error::CreatePool(e) => Error::CreatePool(e),
        Error::GetDbConnection(e) => Error::GetDbConnection(e),
        Error::InitDb(e) => Error::InitDb(e),
        Error::Execute {
            error,
            context,
            location,
        } => Error::Execute {
            error,
            context,
            location,
        },
        Error::Data {
            error,
            context,
            location,
        } => Error::Data {
            error,
            context,
            location,
        },
        Error::Json {
            error,
            context,
            location,
        } => Error::Json {
            error,
            context,
            location,
        },
        Error::ResourceNotFound { kind, id, location } => {
            Error::ResourceNotFound { kind, id, location }
        }
        Error::Io(e) => Error::Io(e),
        Error::NoCert => Error::NoCert,
        Error::AddCert(e) => Error::AddCert(e),
        Error::Deserialize(e) => Error::Deserialize(e),
        Error::Storage(e) => Error::Storage(e),
        Error::Bcrypt => Error::Bcrypt,
        Error::Base58 => Error::Base58,
        Error::LocalStorage {
            error,
            context,
            location,
        } => Error::LocalStorage {
            error,
            context,
            location,
        },
        Error::Parsing {
            error,
            context,
            location,
        } => Error::Parsing {
            error,
            context,
            location,
        },
    }
}

impl UserConnection {
    pub async fn create_apikey(&self, name: &str) -> Result<(APIKey, String), Error<NameConflict>> {
        let mut rng = rand::thread_rng();
        let info = KeyInfo::new(name, self.user_id);

        let conn = self.pool.get_conn().await.map_err(convert_error)?;
        let stmt = conn
            .prepare_cached(
                "INSERT INTO apikeys (
                key_hash,
                user_id,
                name,
                trimmed_key,
                created_at
            ) VALUES ($1, $2, $3, $4, $5)",
            )
            .await
            .map_err(Error::exec("prepare"))?;

        let (key, full_key) = loop {
            let (key, full_key) = APIKey::generate(&mut rng, info.clone());
            let result = conn
                .execute(
                    &stmt,
                    &[
                        &key.key_hash,
                        &key.info.user_id,
                        &key.info.name,
                        &key.trimmed_key,
                        &key.info.created_at,
                    ],
                )
                .await;
            match result {
                Ok(_) => break (key, full_key),
                Err(error) => {
                    if let Some(db_error) = error.as_db_error() {
                        if *db_error.code() == SqlState::UNIQUE_VIOLATION {
                            match db_error.constraint() {
                                Some("uc-user_id-name") => {
                                    return Err(Error::LogicError(NameConflict));
                                }
                                Some("apikeys_pkey") => {
                                    continue;
                                }
                                _ => {}
                            }
                        }
                    }
                    return Err(Error::exec("insert_apikey")(error));
                }
            }
        };

        Ok((key, full_key))
    }

    pub async fn delete_apikey(&self, key_hash: &str) -> crate::Result<()> {
        let conn = self.pool.get_conn().await?;
        let affected = conn
            .do_execute(
                "DELETE FROM apikeys WHERE key_hash = $1 AND user_id = $2",
                &[&key_hash, &self.user_id],
            )
            .await
            .map_err(Error::exec("delete_apikey"))?;
        if affected != 1 {
            return Err(Error::not_found("apikey", key_hash));
        }
        Ok(())
    }
}

pub struct User {
    pub user_id: UserId,
    pub pubkey: [u8; 32],
}

impl AdminConn {
    pub async fn get_user_from_apikey(self, key: &str) -> crate::Result<User> {
        let key_hash =
            base64::encode_config(blake3::hash(key.as_bytes()).as_bytes(), base64::URL_SAFE);
        let conn = self.pool.get_conn().await?;
        let row = conn
            .do_query_one(
                "SELECT
                    users_public.user_id,
                    users_public.pub_key
                FROM apikeys LEFT JOIN users_public
                ON apikeys.user_id = users_public.user_id
                WHERE apikeys.key_hash = $1",
                &[&key_hash],
            )
            .await
            .map_err(Error::exec("get_apikey"))?;
        let user_id: UserId = row.try_get(0).map_err(Error::data("user_id"))?;
        let pubkey = {
            let s: String = row.try_get(1).map_err(Error::data("pub_key"))?;
            let mut buf = [0u8; 32];
            let size = bs58::decode(&s).into(&mut buf).map_err(|_| Error::Base58)?;
            if size != buf.len() {
                return Err(Error::Base58);
            }
            buf
        };

        Ok(User { user_id, pubkey })
    }

    pub async fn get_user_id_from_apikey(self, key: &str) -> crate::Result<UserId> {
        let key_hash =
            base64::encode_config(blake3::hash(key.as_bytes()).as_bytes(), base64::URL_SAFE);
        let conn = self.pool.get_conn().await?;
        let row = conn
            .do_query_one(
                "SELECT user_id FROM apikeys WHERE key_hash = $1",
                &[&key_hash],
            )
            .await
            .map_err(Error::exec("get_apikey"))?;
        let user_id = row.try_get(0).map_err(Error::data("user_id"))?;
        Ok(user_id)
    }
}
