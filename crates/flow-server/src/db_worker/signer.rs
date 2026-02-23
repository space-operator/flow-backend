use actix::{Actor, ResponseFuture};
use db::{Error as DbError, Wallet, pool::DbPool};
use flow_lib::{
    UserId,
    context::signer::{self, SignatureRequest},
};
use futures_util::FutureExt;
use hashbrown::{HashMap, hash_map::Entry};
use serde_json::Value as JsonValue;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use std::future::ready;
use thiserror::Error as ThisError;

pub enum SignerType {
    Keypair(Box<Keypair>),
    UserWallet {
        // Forward to UserWorker
        user_id: UserId,
        sender: actix::Recipient<SignatureRequest>,
    },
}

impl std::fmt::Debug for SignerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keypair(k) => f
                .debug_tuple("SignerType::Keypair")
                .field(&k.pubkey())
                .finish(),
            Self::UserWallet { user_id, .. } => f
                .debug_tuple("SignerType::UserWallet")
                .field(&user_id)
                .finish(),
        }
    }
}

pub struct SignerWorker {
    pub signers: HashMap<Pubkey, SignerType>,
}

impl Actor for SignerWorker {
    type Context = actix::Context<Self>;
}

impl actix::Handler<SignatureRequest> for SignerWorker {
    type Result = ResponseFuture<<SignatureRequest as actix::Message>::Result>;

    fn handle(&mut self, msg: SignatureRequest, _: &mut Self::Context) -> Self::Result {
        match self.signers.get(&msg.pubkey) {
            None => ready(Err(signer::Error::Pubkey(msg.pubkey.to_string()))).boxed(),
            Some(SignerType::Keypair(keypair)) => ready(Ok(signer::SignatureResponse {
                signature: keypair.sign_message(&msg.message),
                new_message: None,
            }))
            .boxed(),
            Some(SignerType::UserWallet { sender, .. }) => {
                let fut = sender.send(msg);
                async move { fut.await? }.boxed()
            }
        }
    }
}

#[derive(ThisError, Debug)]
pub enum AddWalletError {
    #[error("pubkey is not on curve")]
    NotOnCurve,
    #[error("invalid keypair")]
    InvalidKeypair,
}

impl SignerWorker {
    pub async fn fetch<'a, I>(db: DbPool, users: I) -> Result<Self, DbError>
    where
        I: IntoIterator<Item = &'a (UserId, actix::Recipient<SignatureRequest>)>,
    {
        let mut signers = HashMap::new();

        for (user_id, sender) in users {
            let conn = db.get_user_conn(*user_id).await?;
            let wallets = conn.get_wallets().await?;
            for w in wallets {
                let pk = Pubkey::new_from_array(w.pubkey);
                if !pk.is_on_curve() {
                    tracing::warn!("invalid wallet: pubkey is not on curve; id={}", w.id);
                    continue;
                }
                let s = match w.keypair {
                    None => SignerType::UserWallet {
                        user_id: *user_id,
                        sender: sender.clone(),
                    },
                    Some(keypair) => {
                        // check to prevent https://github.com/advisories/GHSA-w5vr-6qhr-36cc
                        if ed25519_dalek::SigningKey::from_keypair_bytes(&keypair).is_err() {
                            tracing::warn!("invalid keypair: mismatch; id={}", w.id);
                            continue;
                        }
                        let keypair = match Keypair::try_from(&keypair[..]) {
                            Ok(keypair) => keypair,
                            Err(error) => {
                                tracing::warn!("invalid keypair: {}; id={}", error, w.id);
                                continue;
                            }
                        };
                        SignerType::Keypair(Box::new(keypair))
                    }
                };
                match signers.entry(pk) {
                    Entry::Vacant(slot) => {
                        slot.insert(s);
                    }
                    Entry::Occupied(mut slot) => {
                        if matches!(
                            (slot.get(), &s),
                            (SignerType::UserWallet { .. }, SignerType::Keypair(_))
                        ) {
                            tracing::warn!("replacing wallet {}", pk);
                            slot.insert(s);
                        }
                    }
                }
            }
        }

        Ok(Self { signers })
    }

    pub async fn fetch_all_and_start<'a, I>(
        db: DbPool,
        users: I,
    ) -> Result<(actix::Addr<Self>, JsonValue), DbError>
    where
        I: IntoIterator<Item = &'a (UserId, actix::Recipient<SignatureRequest>)>,
    {
        let signer = Self::fetch(db, users).await?;
        let signers_info = signer
            .signers
            .iter()
            .map(|(pk, w)| {
                (
                    pk.to_string(),
                    match w {
                        SignerType::Keypair(_) => "HARDCODED".to_owned(),
                        SignerType::UserWallet { user_id, .. } => user_id.to_string(),
                    },
                )
            })
            .collect::<JsonValue>();
        Ok((signer.start(), signers_info))
    }

    pub fn add_wallet(
        &mut self,
        user_id: &UserId,
        sender: &actix::Recipient<SignatureRequest>,
        w: Wallet,
    ) -> Result<(), AddWalletError> {
        let pk = Pubkey::new_from_array(w.pubkey);
        if !pk.is_on_curve() {
            return Err(AddWalletError::NotOnCurve);
        }
        let s = if let Some(keypair) = w.keypair {
            let keypair =
                Keypair::try_from(&keypair[..]).map_err(|_| AddWalletError::InvalidKeypair)?;
            SignerType::Keypair(Box::new(keypair))
        } else {
            SignerType::UserWallet {
                user_id: *user_id,
                sender: sender.clone(),
            }
        };
        match self.signers.entry(pk) {
            Entry::Vacant(slot) => {
                slot.insert(s);
            }
            Entry::Occupied(mut slot) => {
                if matches!(
                    (slot.get(), &s),
                    (SignerType::UserWallet { .. }, SignerType::Keypair(_))
                ) {
                    slot.insert(s);
                }
            }
        }

        Ok(())
    }

    /// Register a keypair for a SWIG session wallet.
    /// The secret key bytes are decoded from base58 and used to create a signing keypair.
    pub fn add_swig_session(
        &mut self,
        pubkey: Pubkey,
        secret_key_base58: &str,
    ) -> Result<(), AddWalletError> {
        let key_bytes = bs58::decode(secret_key_base58)
            .into_vec()
            .map_err(|_| AddWalletError::InvalidKeypair)?;
        let keypair = Keypair::try_from(key_bytes.as_slice())
            .map_err(|_| AddWalletError::InvalidKeypair)?;
        if keypair.pubkey() != pubkey {
            tracing::warn!(
                "swig session keypair mismatch: expected {}, got {}",
                pubkey,
                keypair.pubkey()
            );
            return Err(AddWalletError::InvalidKeypair);
        }
        match self.signers.entry(pubkey) {
            Entry::Vacant(slot) => {
                slot.insert(SignerType::Keypair(Box::new(keypair)));
            }
            Entry::Occupied(mut slot) => {
                // Keypair always wins over UserWallet
                if matches!(slot.get(), SignerType::UserWallet { .. }) {
                    tracing::info!("replacing UserWallet with swig session keypair for {}", pubkey);
                    slot.insert(SignerType::Keypair(Box::new(keypair)));
                }
            }
        }
        Ok(())
    }

    pub async fn fetch_wallets_from_ids(
        db: &DbPool,
        user_id: UserId,
        sender: actix::Recipient<SignatureRequest>,
        ids: &[i64],
    ) -> Result<Self, DbError> {
        let mut signer = Self {
            signers: HashMap::new(),
        };
        let conn = db.get_user_conn(user_id).await?;
        let wallets = conn.get_some_wallets(ids).await?;
        for wallet in wallets {
            if let Err(error) = signer.add_wallet(&user_id, &sender, wallet) {
                tracing::warn!("could not add wallet: {}", error);
            }
        }
        Ok(signer)
    }
}
