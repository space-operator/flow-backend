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
pub struct CreateAuctionHouse {
    pub treasury_mint_account: Option<Option<NodeId>>,
    pub fee_payer: Option<NodeId>,
    pub fee_withdrawal_destination: Option<NodeId>,
    pub auction_house_authority: Option<NodeId>,
    pub treasury_withdrawal_destination: Option<NodeId>,
    pub treasury_withdrawal_destination_owner: Option<NodeId>,
    pub seller_fee_basis_points: Option<u16>,
    pub requires_sign_off: Option<bool>,
    pub can_change_sale_price: Option<bool>,
}

impl CreateAuctionHouse {
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

        let fee_withdrawal_destination = match self.fee_withdrawal_destination {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("fee_withdrawal_destination") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "fee_withdrawal_destination".to_string(),
                    ))
                }
            },
        };

        let auction_house_authority = match self.auction_house_authority {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("auction_house_authority") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "auction_house_authority".to_string(),
                    ))
                }
            },
        };

        let treasury_withdrawal_destination = match self.treasury_withdrawal_destination {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("treasury_withdrawal_destination") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "treasury_withdrawal_destination".to_string(),
                    ))
                }
            },
        };

        let treasury_withdrawal_destination_owner = match self.treasury_withdrawal_destination_owner
        {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("treasury_withdrawal_destination_owner") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "treasury_withdrawal_destination_owner".to_string(),
                    ))
                }
            },
        };

        let seller_fee_basis_points = match self.seller_fee_basis_points {
            Some(s) => s,
            None => match inputs.remove("seller_fee_basis_points") {
                Some(Value::U16(s)) => s,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "seller_fee_basis_points".to_string(),
                    ))
                }
            },
        };

        let requires_sign_off = match self.requires_sign_off {
            Some(s) => s,
            None => match inputs.remove("requires_sign_off") {
                Some(Value::Bool(s)) => s,
                Some(Value::Empty) => false,
                None => false,
                _ => return Err(Error::ArgumentNotFound("requires_sign_off".to_string())),
            },
        };

        let can_change_sale_price = match self.can_change_sale_price {
            Some(s) => s,
            None => match inputs.remove("can_change_sale_price") {
                Some(Value::Bool(s)) => s,
                Some(Value::Empty) => true,
                None => true,
                _ => return Err(Error::ArgumentNotFound("can_change_sale_price".to_string())),
            },
        };

        let treasury_mint_account =
            treasury_mint_account.unwrap_or_else(spl_token::native_mint::id);

        let (minimum_balance_for_rent_exemption, instructions) = command_create_auction_house(
            &ctx.client,
            treasury_mint_account,
            fee_payer.pubkey(),
            fee_withdrawal_destination,
            auction_house_authority,
            treasury_withdrawal_destination,
            treasury_withdrawal_destination_owner,
            seller_fee_basis_points,
            requires_sign_off,
            can_change_sale_price,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&fee_payer];

        let signature = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        let outputs = hashmap! {
            "treasury_withdrawal_destination".to_owned()=> Value::Pubkey(treasury_withdrawal_destination.into()),
            "signature".to_owned() => Value::Success(signature),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "auction_house_authority".to_owned() => Value::Pubkey(auction_house_authority.into()),
            "treasury_mint_account".to_owned() => Value::Pubkey(treasury_mint_account.into()),
        };

        Ok(outputs)
    }
}

pub fn command_create_auction_house(
    rpc_client: &RpcClient,
    treasury_mint_account: Pubkey,
    fee_payer: Pubkey,
    fee_withdrawal_destination: Pubkey,
    auction_house_authority: Pubkey,
    treasury_withdrawal_destination: Pubkey,
    treasury_withdrawal_destination_owner: Pubkey,
    seller_fee_basis_points: u16,
    requires_sign_off: bool,
    can_change_sale_price: bool,
) -> Result<(u64, Vec<Instruction>), Error> {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(mpl_auction_house::AUCTION_HOUSE_SIZE)?;

    let (auction_house_address, bump) = mpl_auction_house::pda::find_auction_house_address(
        &auction_house_authority,
        &treasury_mint_account,
    );

    let (auction_fee_account_key, fee_payer_bump) =
        mpl_auction_house::pda::find_auction_house_fee_account_address(&auction_house_address);

    let (auction_house_treasury_key, treasury_bump) =
        mpl_auction_house::pda::find_auction_house_treasury_address(&auction_house_address);

    let accounts = mpl_auction_house::accounts::CreateAuctionHouse {
        treasury_mint: treasury_mint_account,
        payer: fee_payer,
        authority: auction_house_authority,
        fee_withdrawal_destination,
        treasury_withdrawal_destination,
        treasury_withdrawal_destination_owner,
        // internal/derived
        auction_house: auction_house_address,
        auction_house_fee_account: auction_fee_account_key,
        auction_house_treasury: auction_house_treasury_key,
        token_program: spl_token::id(),
        system_program: solana_sdk::system_program::id(),
        ata_program: spl_associated_token_account::id(),
        rent: solana_sdk::sysvar::rent::id(),
    }
    .to_account_metas(None);

    let data = mpl_auction_house::instruction::CreateAuctionHouse {
        _bump: bump,
        fee_payer_bump,
        treasury_bump,
        seller_fee_basis_points,
        requires_sign_off,
        can_change_sale_price,
    }
    .data();

    let instruction = Instruction {
        program_id: mpl_auction_house::id(),
        data,
        accounts,
    };

    let instructions = vec![instruction];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
