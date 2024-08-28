use crate::SupabaseConfig;
use bincode::{Decode, Encode};
use db::pool::{DbPool, RealDbPool};
use flow::BoxedError;
use flow_lib::{FlowRunId, UserId};
use reqwest::header::{self, HeaderName, HeaderValue};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::panic::Location;
use thiserror::Error as ThisError;

pub const FLOW_RUN_TOKEN_PREFIX: &str = "fr-";
pub const SIGNING_TIMEOUT_SECS: i64 = 60;
const HEADER: &str = "space-operator authentication\n\n";

#[derive(Clone, Copy)]
pub struct SignatureAuth {
    secret: [u8; blake3::KEY_LEN],
}

#[derive(Encode, Decode)]
pub struct Payload {
    pubkey: [u8; 32],
    timestamp: i64,
}

fn bincode_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_fixed_int_encoding()
        .skip_fixed_array_length()
}

#[derive(ThisError, Debug)]
#[error("signature verification failed")]
pub struct Invalid(&'static Location<'static>);

#[track_caller]
fn invalid() -> Invalid {
    Invalid(std::panic::Location::caller())
}

impl SignatureAuth {
    pub fn new(secret: [u8; 32]) -> Self {
        Self { secret }
    }

    pub(crate) fn hash(&self, data: &[u8]) -> blake3::Hash {
        blake3::keyed_hash(&self.secret, data)
    }

    /// `fr-` + `base64(id + hash(id))`
    pub fn flow_run_token(&self, id: FlowRunId) -> String {
        let mut buf = Vec::<u8>::with_capacity(48);
        buf.extend_from_slice(id.as_bytes());
        let hash = blake3::keyed_hash(&self.secret, &buf);
        buf.extend_from_slice(hash.as_bytes());
        let mut msg = FLOW_RUN_TOKEN_PREFIX.to_owned();
        base64::encode_config_buf(&buf, base64::URL_SAFE_NO_PAD, &mut msg);
        msg
    }

    pub fn init_login(&self, now: i64, pubkey: &[u8; 32]) -> String {
        let payload = Payload {
            pubkey: *pubkey,
            timestamp: now,
        };
        let mut bytes = Vec::with_capacity(72);
        bincode::encode_into_std_write(&payload, &mut bytes, bincode_config()).unwrap();
        let sig = blake3::keyed_hash(&self.secret, &bytes);
        bytes.extend_from_slice(sig.as_bytes());
        let mut msg = HEADER.to_owned();
        base64::encode_config_buf(&bytes, base64::URL_SAFE_NO_PAD, &mut msg);
        msg
    }

    /// `<signed payload>.<ed25519 signature>`
    pub fn confirm(&self, now: i64, input: &str) -> Result<Payload, Invalid> {
        if !input.starts_with(HEADER) {
            return Err(invalid());
        }

        let (signed_payload, sig) = input.split_once('.').ok_or_else(invalid)?;

        let signed_payload_bytes = base64::decode_config(
            signed_payload.strip_prefix(HEADER).unwrap(),
            base64::URL_SAFE,
        )
        .map_err(|_| invalid())?;
        let split_pos = signed_payload_bytes
            .len()
            .checked_sub(32)
            .ok_or_else(invalid)?;
        let (payload_bytes, blake3_sig) = signed_payload_bytes.split_at(split_pos);
        let (payload, size) =
            bincode::decode_from_slice::<Payload, _>(payload_bytes, bincode_config())
                .map_err(|_| invalid())?;
        if size != payload_bytes.len() {
            return Err(invalid());
        }
        if now - payload.timestamp >= SIGNING_TIMEOUT_SECS {
            return Err(invalid());
        }
        if blake3::keyed_hash(&self.secret, payload_bytes) != *blake3_sig {
            return Err(invalid());
        }
        let mut signature = [0u8; 64];
        let size = bs58::decode(sig)
            .into(&mut signature)
            .map_err(|_| invalid())?;
        if size != 64 {
            return Err(invalid());
        }
        let signature = ed25519_dalek::Signature::from_bytes(&signature);
        let pubkey =
            ed25519_dalek::VerifyingKey::from_bytes(&payload.pubkey).map_err(|_| invalid())?;
        pubkey
            .verify_strict(signed_payload.as_bytes(), &signature)
            .map_err(|_| invalid())?;
        Ok(payload)
    }
}

#[derive(Clone)]
pub struct SupabaseAuth {
    client: reqwest::Client,
    pool: RealDbPool,
    anon_key: String,
    login_url: Url,
    create_user_url: Url,
    admin_token: HeaderValue,
    open_whitelists: bool,
}

#[derive(ThisError, Debug)]
pub enum LoginError {
    #[error("login error")]
    Failed(&'static Location<'static>),
    #[error(transparent)]
    Db(#[from] db::Error),
    #[error(transparent)]
    Supabase(SupabaseError),
}

#[derive(Deserialize, ThisError, Debug)]
#[error("{msg}")]
pub struct SupabaseError {
    pub msg: String,
}

async fn supabase_error(resp: reqwest::Response) -> LoginError {
    let bytes = match resp.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!("network error: {}", error);
            return login_error();
        }
    };
    match serde_json::from_slice::<SupabaseError>(&bytes) {
        Ok(msg) => LoginError::Supabase(msg),
        Err(error) => {
            tracing::warn!("decode error: {}", error);
            tracing::warn!("error body: {}", String::from_utf8_lossy(&bytes));
            login_error()
        }
    }
}

#[track_caller]
fn login_error() -> LoginError {
    LoginError::Failed(std::panic::Location::caller())
}

#[derive(Serialize)]
pub struct PasswordLogin {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct CreateUser {
    email: String,
    email_confirm: bool,
    user_metadata: UserMetadata,
}

#[derive(Serialize)]
struct UserMetadata {
    pub_key: String,
}

pub fn get_email(pubkey: &[u8; 32]) -> String {
    hex::encode(pubkey) + "@spaceoperator.com"
}

impl CreateUser {
    fn new(pk: &[u8; 32]) -> Self {
        let pub_key = bs58::encode(pk).into_string();
        let email = get_email(pk);
        Self {
            email,
            email_confirm: true,
            user_metadata: UserMetadata { pub_key },
        }
    }
}

#[derive(Deserialize)]
struct CreateUserResponse {
    id: UserId,
}

impl SupabaseAuth {
    pub fn new(config: &SupabaseConfig, pool: DbPool) -> Result<Self, BoxedError> {
        let pool = match pool {
            DbPool::Real(pool) => pool,
            _ => return Err("need database credentials".into()),
        };
        let base_url = config.endpoint.url.join("auth/v1/")?;
        let service_key = config.service_key.as_ref().ok_or("need service_key")?;
        let login_url = base_url.join("token?grant_type=password")?;
        let create_user_url = base_url.join("admin/users")?;
        let admin_token = HeaderValue::from_str(&format!("Bearer {}", service_key))?;

        Ok(Self {
            client: reqwest::Client::new(),
            anon_key: config.anon_key.clone(),
            login_url,
            create_user_url,
            admin_token,
            pool,
            open_whitelists: config.open_whitelists,
        })
    }

    pub async fn get_or_create_user(
        &self,
        pubkey: &[u8; 32],
    ) -> Result<(UserId, bool), LoginError> {
        let conn = self.pool.get_admin_conn().await?;
        let pk_bs58 = bs58::encode(pubkey).into_string();
        let maybe_user = conn.get_user_id_by_pubkey(&pk_bs58).await?;
        if let Some(user_id) = maybe_user {
            return Ok((user_id, false));
        }

        tracing::info!("creating user {}", pk_bs58);
        if self.open_whitelists {
            conn.insert_whitelist(&pk_bs58).await?;
        }
        drop(conn);

        let resp = self
            .client
            .post(self.create_user_url.clone())
            .header(HeaderName::from_static("apikey"), &self.anon_key)
            .header(header::AUTHORIZATION, &self.admin_token)
            .json(&CreateUser::new(pubkey))
            .send()
            .await
            .map_err(|_| login_error())?;
        if resp.status() != StatusCode::OK {
            return Err(supabase_error(resp).await);
        }
        let CreateUserResponse { id } = resp.json().await.map_err(|_| login_error())?;

        Ok((id, true))
    }

    pub async fn login(&self, payload: &Payload) -> Result<(Box<RawValue>, bool), LoginError> {
        let pk = bs58::encode(&payload.pubkey).into_string();
        tracing::info!("login {}", pk);

        let (user_id, new_user) = self.get_or_create_user(&payload.pubkey).await?;
        let r = self
            .pool
            .get_admin_conn()
            .await?
            .get_login_credential(user_id)
            .await?;

        let body = PasswordLogin {
            email: r.email,
            password: r.password,
        };

        tracing::debug!("calling supabase login");
        let resp = self
            .client
            .post(self.login_url.clone())
            .header(HeaderName::from_static("apikey"), &self.anon_key)
            .json(&body)
            .send()
            .await
            .map_err(|_| login_error())?;
        if resp.status() != StatusCode::OK {
            return Err(supabase_error(resp).await);
        }

        let body: Box<RawValue> = resp.json().await.map_err(|_| login_error())?;

        Ok((body, new_user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    #[test]
    fn test_sign_verify() {
        let kp = ed25519_dalek::SigningKey::from_bytes(&rand::random::<[u8; 32]>());
        let m = SignatureAuth::new(rand::random());
        let msg = m.init_login(now(), kp.verifying_key().as_bytes());
        let signature = bs58::encode(&kp.sign(msg.as_bytes()).to_bytes()).into_string();
        m.confirm(now(), &format!("{msg}.{signature}")).unwrap();
    }
}
