use flow_lib::solana::Pubkey;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack};

pub mod associated_token_account;
// pub mod clockwork;
pub mod compression;
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
pub mod request_airdrop;
pub mod transfer_sol;
pub mod transfer_token;
pub mod utils;
pub mod wallet;
pub mod wormhole;
// pub mod xnft;
pub mod das;
pub mod streamflow;
pub mod pyth;

pub use error::{Error, Result};
use tracing::info;

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

// make a nodes out of this
pub async fn get_decimals(client: &RpcClient, mint_account: Pubkey) -> crate::Result<u8> {
    let commitment = CommitmentConfig::confirmed();
    info!("commitment: {:?}", commitment);

    let response = client
        .get_account_with_commitment(&mint_account, commitment)
        .await
        .map_err(|e| {
            tracing::error!("Error: {:?}", e);
            crate::Error::AccountNotFound(mint_account)
        })?;
    info!("response: {:?}", response);

    let source_account = match response.value {
        Some(account) => account,
        None => return Err(crate::Error::AccountNotFound(mint_account)),
    };

    // let source_account = client.get_account(&mint_account).await.map_err(|e| {
    //     tracing::error!("Error: {:?}", e);
    //     crate::Error::AccountNotFound(mint_account)
    // })?;
    let source_account = spl_token::state::Mint::unpack(&source_account.data)?;
    info!("source_account: {:?}", source_account);
    Ok(source_account.decimals)
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
