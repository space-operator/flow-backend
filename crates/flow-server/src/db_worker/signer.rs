use actix::{Actor, ResponseFuture};
use flow_lib::context::signer;
use futures_util::FutureExt;
use hashbrown::HashMap;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::future::ready;

pub enum SignerType {
    Keypair(Keypair),
    UserWallet {
        // Forward to UserWorker
        sender: actix::Recipient<signer::SignatureRequest>,
    },
}

pub struct SignerWorker {
    pub signers: HashMap<Pubkey, SignerType>,
}

impl Actor for SignerWorker {
    type Context = actix::Context<Self>;
}

impl actix::Handler<signer::SignatureRequest> for SignerWorker {
    type Result = ResponseFuture<<signer::SignatureRequest as actix::Message>::Result>;

    fn handle(&mut self, msg: signer::SignatureRequest, _: &mut Self::Context) -> Self::Result {
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
