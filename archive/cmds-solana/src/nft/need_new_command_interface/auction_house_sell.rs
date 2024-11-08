use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use anchor_lang::{InstructionData, ToAccountMetas};
use dashmap::DashMap;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, signer::Signer};
use spl_token::instruction::transfer_checked;
use uuid::Uuid;

use sunshine_core::msg::NodeId;

use crate::commands::solana::instructions::execute;
use crate::commands::solana::SolanaNet;
use crate::{Error, NftMetadata, Value};

use solana_sdk::signer::keypair::write_keypair_file;

use bundlr_sdk::{tags::Tag, Bundlr, Signer as BundlrSigner, SolanaSigner};

use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuctionHouseSell {
    pub treasury_mint_account: Option<Option<NodeId>>,
    pub fee_payer: Option<NodeId>,
    pub auction_house_authority: Option<NodeId>,
    pub seller: Option<NodeId>,
    pub seller_token_account: Option<NodeId>,
    pub seller_token_mint_account: Option<NodeId>,
    pub sale_price: Option<u64>,
}

impl AuctionHouseSell {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let treasury_mint_account = match self.treasury_mint_account {
            Some(s) => match s {
                Some(treasury_mint_account) => {
                    Some(ctx.get_pubkey_by_id(treasury_mint_account).await?)
                }
                None => None,
            },
            None => match inputs.remove("treasury_mint_account") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(treasury_mint_account) => {
                        Some(ctx.get_pubkey_by_id(treasury_mint_account).await?)
                    }
                    None => None,
                },
                Some(Value::Keypair(k)) => Some(Keypair::from(k).pubkey()),
                Some(Value::Pubkey(p)) => Some(p.into()),
                Some(Value::Empty) => None,
                None => None,
                _ => return Err(Error::ArgumentNotFound("treasury_mint_account".to_string())),
            },
        };

        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let auction_house_authority = match self.auction_house_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("auction_house_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "auction_house_authority".to_string(),
                    ))
                }
            },
        };

        let seller = match self.seller {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("seller") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("seller".to_string())),
            },
        };

        let seller_token_account = match self.seller_token_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("seller_token_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("seller_token_account".to_string())),
            },
        };

        let seller_token_mint_account = match self.seller_token_mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("seller_token_mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "seller_token_mint_account".to_string(),
                    ))
                }
            },
        };

        let sale_price = match self.sale_price {
            Some(s) => s,
            None => match inputs.remove("sale_price") {
                Some(Value::U64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("sale_price".to_string())),
            },
        };

        let treasury_mint_account =
            treasury_mint_account.unwrap_or_else(spl_token::native_mint::id);

        let (minimum_balance_for_rent_exemption, instructions) = command_auction_house_sell(
            &ctx.client,
            treasury_mint_account,
            auction_house_authority.pubkey(),
            seller.pubkey(),
            seller_token_account,
            seller_token_mint_account,
            sale_price,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&fee_payer, &seller];

        let signature = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        let outputs = hashmap! {
            "signature".to_owned() => Value::Success(signature),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "auction_house_authority".to_owned() => Value::Keypair(auction_house_authority.into()),
            "treasury_mint_account".to_owned() => Value::Pubkey(treasury_mint_account.into()),
        };

        Ok(outputs)
    }
}

pub fn command_auction_house_sell(
    rpc_client: &RpcClient,
    treasury_mint_account: Pubkey,
    auction_house_authority: Pubkey,
    seller: Pubkey,
    seller_token_account: Pubkey,
    seller_token_mint_account: Pubkey,
    sale_price: u64,
) -> Result<(u64, Vec<Instruction>), Error> {
    let minimum_balance_for_rent_exemption = rpc_client.get_minimum_balance_for_rent_exemption(
        mpl_auction_house::TRADE_STATE_SIZE + mpl_auction_house::receipt::LISTING_RECEIPT_SIZE,
    )?;

    let (seller_token_metadata_account, _) =
        mpl_token_metadata::pda::find_metadata_account(&seller_token_mint_account);

    let program_id = mpl_auction_house::id();

    let (auction_house_address, bump) = mpl_auction_house::pda::find_auction_house_address(
        &auction_house_authority,
        &treasury_mint_account,
    );

    let (auction_fee_account_key, _) =
        mpl_auction_house::pda::find_auction_house_fee_account_address(&auction_house_address);

    let (seller_trade_state, sts_bump) = mpl_auction_house::pda::find_trade_state_address(
        &seller,
        &auction_house_address,
        &seller_token_account,
        &treasury_mint_account,
        &seller_token_mint_account,
        sale_price,
        1,
    );

    let (free_seller_trade_state, free_sts_bump) = mpl_auction_house::pda::find_trade_state_address(
        &seller,
        &auction_house_address,
        &seller_token_account,
        &treasury_mint_account,
        &seller_token_mint_account,
        0,
        1,
    );

    let (listing_receipt, receipt_bump) =
        mpl_auction_house::pda::find_listing_receipt_address(&seller_trade_state);

    let (program_as_signer, pas_bump) = mpl_auction_house::pda::find_program_as_signer_address();

    let accounts = mpl_auction_house::accounts::Sell {
        wallet: seller,
        token_account: seller_token_account,
        metadata: seller_token_metadata_account,
        authority: auction_house_authority,
        auction_house: auction_house_address,
        auction_house_fee_account: auction_fee_account_key,
        seller_trade_state,
        free_seller_trade_state,
        token_program: spl_token::id(),
        system_program: solana_sdk::system_program::id(),
        program_as_signer,
        rent: solana_sdk::sysvar::rent::id(),
    }
    .to_account_metas(None);

    let data = mpl_auction_house::instruction::Sell {
        trade_state_bump: sts_bump,
        _free_trade_state_bump: free_sts_bump,
        _program_as_signer_bump: pas_bump,
        token_size: 1,
        buyer_price: sale_price,
    }
    .data();

    let sell_instruction = Instruction {
        program_id,
        data,
        accounts,
    };

    let listing_receipt_accounts = mpl_auction_house::accounts::PrintListingReceipt {
        receipt: listing_receipt,
        bookkeeper: seller,
        system_program: solana_sdk::system_program::id(),
        rent: solana_sdk::sysvar::rent::id(),
        instruction: solana_sdk::sysvar::instructions::id(),
    };

    let print_receipt_instruction = Instruction {
        program_id,
        data: mpl_auction_house::instruction::PrintListingReceipt { receipt_bump }.data(),
        accounts: listing_receipt_accounts.to_account_metas(None),
    };

    let instructions = vec![sell_instruction, print_receipt_instruction];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
