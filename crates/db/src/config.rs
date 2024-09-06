use chacha20poly1305::{aead::Aead, AeadCore, ChaCha20Poly1305, KeyInit};
use flow_lib::solana::Keypair;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::fmt::Display;
use zeroize::Zeroize;

#[serde_as]
#[derive(Deserialize, Clone)]
pub(crate) struct EncryptionKey(#[serde_as(as = "Base64")] [u8; 32]);

impl Drop for EncryptionKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Encrypted {
    #[serde_as(as = "Base64")]
    #[serde(rename = "n")]
    pub nonce: [u8; 12],
    #[serde_as(as = "Base64")]
    #[serde(rename = "c")]
    pub ciphertext: Vec<u8>,
}

impl EncryptionKey {
    pub(crate) fn encrypt(&self, plaintext: &[u8]) -> Encrypted {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.0).expect("we use correct length");
        let nonce = ChaCha20Poly1305::generate_nonce(&mut rand::thread_rng());
        let ciphertext = cipher.encrypt(&nonce, plaintext).unwrap();
        Encrypted {
            nonce: nonce.try_into().unwrap(),
            ciphertext,
        }
    }

    pub(crate) fn decrypt(
        &self,
        encrypted: &Encrypted,
    ) -> Result<Vec<u8>, chacha20poly1305::Error> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.0).expect("we use correct length");
        cipher.decrypt((&encrypted.nonce).into(), encrypted.ciphertext.as_ref())
    }

    pub(crate) fn encrypt_keypair(&self, keypair: &Keypair) -> Encrypted {
        self.encrypt(keypair.secret().as_bytes())
    }

    pub(crate) fn decrypt_keypair(
        &self,
        encrypted: &Encrypted,
    ) -> Result<Keypair, chacha20poly1305::Error> {
        let mut secret = self.decrypt(encrypted)?;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(
            &secret
                .as_slice()
                .try_into()
                .map_err(|_| chacha20poly1305::Error)?,
        );
        secret.zeroize();
        Keypair::from_bytes(&signing_key.to_keypair_bytes()).map_err(|_| chacha20poly1305::Error)
    }
}

#[derive(Deserialize)]
pub struct DbConfig {
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub ssl: SslConfig,
    pub max_pool_size: Option<usize>,
    pub(crate) encryption_key: Option<EncryptionKey>,
}

#[derive(Deserialize, Clone, Default)]
pub struct SslConfig {
    pub enabled: bool,
    pub cert: Option<std::path::PathBuf>,
}

impl Display for DbConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, self.dbname,
        ))
    }
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            user: "postgres".into(),
            password: "spacepass".into(),
            dbname: "space-operator-db".into(),
            host: "127.0.0.1".into(),
            port: 7979,
            ssl: <_>::default(),
            max_pool_size: None,
            encryption_key: None,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ProxiedDbConfig {
    pub upstream_url: String,
    pub api_keys: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption() {
        let mut rng = rand::thread_rng();
        let key = EncryptionKey(ChaCha20Poly1305::generate_key(&mut rng).into());
        let keypair =
            Keypair::from_bytes(&ed25519_dalek::SigningKey::generate(&mut rng).to_keypair_bytes())
                .unwrap();
        let decrypted = key.decrypt_keypair(&key.encrypt_keypair(&keypair)).unwrap();
        assert_eq!(keypair, decrypted);
    }
}
