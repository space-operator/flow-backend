pub mod associated_token_account;
// pub mod clockwork;
// pub mod compression;
pub mod create_mint_account;
pub mod create_token_account;
pub mod error;
pub mod find_pda;
pub mod generate_keypair;
pub mod get_balance;
// pub mod metaboss;
pub mod mint_token;
pub mod nft;
// pub mod proxy_authority;
pub mod http_request;
pub mod request_airdrop;
pub mod std;
pub mod transfer_sol;
pub mod transfer_token;
pub mod utils;
pub mod wallet;
pub mod wormhole;
// pub mod xnft;
pub mod db;

pub use error::{Error, Result};

pub mod prelude {
    pub use crate::utils::{execute, submit_transaction, try_sign_wallet};
    pub use async_trait::async_trait;
    pub use flow_lib::{
        command::{
            builder::{BuildResult, BuilderCache, BuilderError, CmdBuilder},
            CommandDescription, CommandError, CommandTrait, InstructionInfo,
        },
        context::Context,
        solana::{Instructions, KeypairExt},
        CmdInputDescription as CmdInput, CmdOutputDescription as CmdOutput, Name, SolanaNet,
        ValueSet, ValueType,
    };
    pub use rust_decimal::Decimal;
    pub use serde::{Deserialize, Serialize};
    pub use solana_client::nonblocking::rpc_client::RpcClient;
    pub use solana_sdk::{
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::Signer,
    };
    pub use std::sync::Arc;
    pub use value::{HashMap, Value};
}

#[cfg(test)]
pub mod tests {
    use crate::prelude::*;

    #[test]
    fn test_name_unique() {
        let mut m = std::collections::HashSet::new();
        let mut dup = false;
        for CommandDescription { name, .. } in inventory::iter::<CommandDescription>() {
            if !m.insert(name) {
                println!("Dupicated: {}", name);
                dup = true;
            }
        }
        assert!(!dup);
    }
}
