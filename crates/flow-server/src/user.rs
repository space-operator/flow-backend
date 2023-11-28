use crate::SupabaseConfig;
use actix_web::http::header::{self, HeaderName, HeaderValue};
use bincode::{Decode, Encode};
use db::{
    connection::Password,
    pool::{DbPool, RealDbPool},
};
use flow::BoxedError;
use rand::distributions::{Alphanumeric, DistString};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::panic::Location;
use thiserror::Error as ThisError;
use uuid::Uuid;

const HEADER: &str = "space-operator authentication\n\n";

#[derive(Clone, Copy)]
pub struct SignatureAuth {
    secret: [u8; 32],
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
        base64::encode_config_buf(&bytes, base64::URL_SAFE, &mut msg);
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
        if now - payload.timestamp >= 20 {
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
        let signature = ed25519_dalek::Signature::from_bytes(&signature).map_err(|_| invalid())?;
        let pubkey =
            ed25519_dalek::PublicKey::from_bytes(&payload.pubkey).map_err(|_| invalid())?;
        pubkey
            .verify_strict(signed_payload.as_bytes(), &signature)
            .map_err(|_| invalid())?;
        Ok(payload)
    }
}

fn rand_password() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 24)
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

impl CreateUser {
    fn new(pk: &[u8; 32]) -> Self {
        let pub_key = bs58::encode(pk).into_string();
        let email = hex::encode(pk) + "@spaceoperator.com";
        Self {
            email,
            email_confirm: true,
            user_metadata: UserMetadata { pub_key },
        }
    }
}

#[derive(Deserialize)]
struct CreateUserResponse {
    id: Uuid,
}

impl SupabaseAuth {
    pub fn new(config: &SupabaseConfig, pool: DbPool) -> Result<Self, BoxedError> {
        let pool = match pool {
            DbPool::Real(pool) => pool,
            _ => return Err("need database credentials".into()),
        };
        let base_url = Url::parse(&format!(
            "https://{}.supabase.co/auth/v1/",
            config.project_id,
        ))?;
        let service_key = config
            .service_key
            .as_ref()
            .ok_or_else(|| "need service_key")?;
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

    pub async fn login(&self, payload: &Payload) -> Result<(Box<RawValue>, bool), LoginError> {
        let pk = bs58::encode(&payload.pubkey).into_string();

        let (cred, new_user) = match self.get_or_reset_password(&pk).await? {
            Some(pw) => (pw, false),
            None => (self.create_user(&payload.pubkey).await?, true),
        };

        let resp = self
            .client
            .post(self.login_url.clone())
            .header(HeaderName::from_static("apikey"), &self.anon_key)
            .json(&cred)
            .send()
            .await
            .map_err(|_| login_error())?;
        if resp.status() != StatusCode::OK {
            return Err(supabase_error(resp).await);
        }

        let body: Box<RawValue> = resp.json().await.map_err(|_| login_error())?;

        Ok((body, new_user))
    }

    pub async fn get_or_reset_password(
        &self,
        pk: &str,
    ) -> Result<Option<PasswordLogin>, LoginError> {
        let mut conn = self
            .pool
            .get_admin_conn()
            .await
            .map_err(|_| login_error())?;
        match conn.get_password(pk).await? {
            Some(Password {
                user_id,
                email,
                password,
            }) => {
                let password = match password {
                    Some(pw) => pw,
                    None => {
                        tracing::info!("resetting password of {}", user_id);
                        let pw = rand_password();
                        conn.reset_password(&user_id, &pw).await?;
                        pw
                    }
                };
                Ok(Some(PasswordLogin { email, password }))
            }
            None => Ok(None),
        }
    }

    pub async fn create_user(&self, pk: &[u8; 32]) -> Result<PasswordLogin, LoginError> {
        let body = CreateUser::new(pk);
        let mut conn = self
            .pool
            .get_admin_conn()
            .await
            .map_err(|_| login_error())?;
        tracing::info!("creating user {}", body.user_metadata.pub_key);
        if self.open_whitelists {
            conn.insert_whitelist(&body.user_metadata.pub_key).await?;
        }
        let resp = self
            .client
            .post(self.create_user_url.clone())
            .header(HeaderName::from_static("apikey"), &self.anon_key)
            .header(header::AUTHORIZATION, &self.admin_token)
            .json(&body)
            .send()
            .await
            .map_err(|_| login_error())?;
        if resp.status() != StatusCode::OK {
            return Err(supabase_error(resp).await);
        }
        let CreateUserResponse { id } = resp.json().await.map_err(|_| login_error())?;

        let pw = rand_password();
        conn.reset_password(&id, &pw).await?;

        Ok(PasswordLogin {
            email: body.email,
            password: pw,
        })
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
        let sk = ed25519_dalek::SecretKey::from_bytes(&rand::random::<[u8; 32]>()).unwrap();
        let kp = ed25519_dalek::Keypair {
            public: (&sk).into(),
            secret: sk,
        };
        let m = SignatureAuth::new(rand::random());
        let msg = m.init_login(now(), kp.public.as_bytes());
        let signature = bs58::encode(kp.sign(msg.as_bytes())).into_string();
        m.confirm(now(), &format!("{msg}.{signature}")).unwrap();
    }
}
