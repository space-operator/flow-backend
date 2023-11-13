use actix::{Actor, ResponseFuture};
use db::{pool::DbPool, Error as DbError};
use flow_lib::{
    context::signer::{self, SignatureRequest},
    UserId,
};
use futures_util::FutureExt;
use hashbrown::HashMap;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::future::ready;

pub enum SignerType {
    Keypair(Keypair),
    UserWallet {
        // Forward to UserWorker
        sender: actix::Recipient<SignatureRequest>,
    },
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
            }))
            .boxed(),
            Some(SignerType::UserWallet { sender }) => {
                let fut = sender.send(msg);
                async move { fut.await? }.boxed()
            }
        }
    }
}

impl SignerWorker {
    pub async fn fetch_and_start<'a, I>(db: DbPool, users: I) -> Result<actix::Addr<Self>, DbError>
    where
        I: IntoIterator<Item = &'a (UserId, actix::Recipient<SignatureRequest>)>,
    {
        let mut signers = HashMap::new();
        for (user, sender) in users {
            let conn = db.get_user_conn(*user).await?;
            let wallets = conn.get_wallets().await?;
            for w in wallets {
                let pk = Pubkey::new_from_array(w.pubkey);
                if !pk.is_on_curve() {
                    tracing::warn!("invalid wallet");
                    continue;
                }
                let s = match w.keypair {
                    None => SignerType::UserWallet {
                        sender: sender.clone(),
                    },
                    Some(keypair) => {
                        let keypair = Keypair::from_bytes(&keypair).ok().and_then(|k| {
                            let pubkey: ed25519_dalek::PublicKey = k.secret().into();
                            (k.pubkey().to_bytes() == pubkey.to_bytes())
                                .then_some(SignerType::Keypair(k))
                        });
                        match keypair {
                            None => {
                                tracing::warn!("invalid wallet");
                                continue;
                            }
                            Some(signer) => signer,
                        }
                    }
                };
                signers.insert(pk, s);
            }
        }
        Ok(Self { signers }.start())
    }
}
