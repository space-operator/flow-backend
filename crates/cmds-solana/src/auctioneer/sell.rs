use super::{AUCTIONEER_BUYER_PRICE, DISC_SELL, build_auctioneer_instruction, pda};
use crate::auction_house::{AUCTION_HOUSE_PROGRAM_ID, TOKEN_PROGRAM_ID, pda as ah_pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;

const NAME: &str = "auctioneer_sell";
const DEFINITION: &str = flow_lib::node_definition!("auctioneer/sell.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub wallet: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    pub token_size: u64,
    /// Auction start (unix seconds).
    pub start_time: i64,
    /// Auction end (unix seconds).
    pub end_time: i64,
    #[serde(default)]
    pub reserve_price: Option<u64>,
    #[serde(default)]
    pub min_bid_increment: Option<u64>,
    /// Trailing window (seconds) before end_time in which a new high bid triggers an extension.
    #[serde(default)]
    pub time_ext_period: Option<u32>,
    /// How many seconds to add to end_time when a late bid triggers an extension.
    #[serde(default)]
    pub time_ext_delta: Option<u32>,
    #[serde(default)]
    pub allow_high_bid_cancel: Option<bool>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub auction_house: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub listing_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seller_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub free_seller_trade_state: Pubkey,
}

fn push_opt_u64(v: Option<u64>, out: &mut Vec<u8>) {
    match v {
        None => out.push(0),
        Some(x) => {
            out.push(1);
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
}
fn push_opt_u32(v: Option<u32>, out: &mut Vec<u8>) {
    match v {
        None => out.push(0),
        Some(x) => {
            out.push(1);
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
}
fn push_opt_bool(v: Option<bool>, out: &mut Vec<u8>) {
    match v {
        None => out.push(0),
        Some(x) => {
            out.push(1);
            out.push(x as u8);
        }
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) = ah_pda::find_auction_house(&input.authority, &input.treasury_mint);
    let (auctioneer_authority, aa_bump) = pda::find_auctioneer_authority(&auction_house);
    let (ah_auctioneer_pda, _) =
        ah_pda::find_ah_auctioneer_pda(&auction_house, &auctioneer_authority);
    let (token_account, _) = ah_pda::find_ata(&wallet_pk, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (metadata, _) = ah_pda::find_metadata(&input.token_mint);
    let (fee_acc, _) = ah_pda::find_auction_house_fee_account(&auction_house);
    let (seller_trade_state, ts_bump) = ah_pda::find_trade_state(
        &wallet_pk,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        AUCTIONEER_BUYER_PRICE,
        input.token_size,
    );
    let (free_seller_trade_state, fts_bump) = ah_pda::find_free_trade_state(
        &wallet_pk,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );
    let (program_as_signer, pas_bump) = ah_pda::find_program_as_signer();
    let (listing_config, _) = pda::find_listing_config(
        &wallet_pk,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );

    let accounts = vec![
        AccountMeta::new_readonly(AUCTION_HOUSE_PROGRAM_ID, false),
        AccountMeta::new(listing_config, false),
        AccountMeta::new(wallet_pk, true),
        AccountMeta::new(token_account, false),
        AccountMeta::new_readonly(metadata, false),
        AccountMeta::new_readonly(input.authority, false),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new(seller_trade_state, false),
        AccountMeta::new(free_seller_trade_state, false),
        AccountMeta::new_readonly(auctioneer_authority, false),
        AccountMeta::new_readonly(ah_auctioneer_pda, false),
        AccountMeta::new_readonly(program_as_signer, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let mut args = Vec::with_capacity(64);
    args.push(ts_bump);
    args.push(fts_bump);
    args.push(pas_bump);
    args.push(aa_bump);
    args.extend_from_slice(&input.token_size.to_le_bytes());
    args.extend_from_slice(&input.start_time.to_le_bytes());
    args.extend_from_slice(&input.end_time.to_le_bytes());
    push_opt_u64(input.reserve_price, &mut args);
    push_opt_u64(input.min_bid_increment, &mut args);
    push_opt_u32(input.time_ext_period, &mut args);
    push_opt_u32(input.time_ext_delta, &mut args);
    push_opt_bool(input.allow_high_bid_cancel, &mut args);

    let ix = build_auctioneer_instruction(DISC_SELL, accounts, args);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.wallet.clone()]
            .into_iter()
            .collect(),
        instructions: vec![ix],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        auction_house,
        listing_config,
        token_account,
        seller_trade_state,
        free_seller_trade_state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        let pk = "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9";
        let kp = "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ";
        let input = value::map! {
            "fee_payer" => kp, "wallet" => kp,
            "authority" => pk, "token_mint" => pk, "treasury_mint" => pk,
            "token_size" => 1u64,
            "start_time" => 1_000_000i64,
            "end_time" => 1_000_060i64,
            "reserve_price" => 100u64,
            "min_bid_increment" => 10u64,
            "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_opt_encoding() {
        let mut v = Vec::new();
        push_opt_u64(None, &mut v);
        push_opt_u64(Some(500), &mut v);
        push_opt_u32(Some(30), &mut v);
        push_opt_bool(Some(true), &mut v);
        // None(u64)=[0]; Some(500)=[1,0xf4,0x01,0,0,0,0,0,0]; Some(30_u32)=[1,0x1e,0,0,0]; Some(true)=[1,1]
        assert_eq!(v[0], 0);
        assert_eq!(v[1], 1);
        assert_eq!(&v[2..10], &500u64.to_le_bytes());
        assert_eq!(v[10], 1);
        assert_eq!(&v[11..15], &30u32.to_le_bytes());
        assert_eq!(v[15], 1);
        assert_eq!(v[16], 1);
    }
}
