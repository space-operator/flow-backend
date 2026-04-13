use super::{AUCTIONEER_PROGRAM_ID, DISC_CANCEL, build_auctioneer_instruction, pda};
use crate::auction_house::{AUCTION_HOUSE_PROGRAM_ID, TOKEN_PROGRAM_ID, pda as ah_pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auctioneer_cancel";
const DEFINITION: &str = flow_lib::node_definition!("auctioneer/cancel.jsonc");

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
    /// The trade state owner (bidder cancelling a bid, or seller cancelling the listing).
    pub wallet: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub authority: Pubkey,
    /// Seller of the NFT (used to locate listing_config; equals wallet when seller cancels).
    #[serde_as(as = "AsPubkey")]
    pub seller: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    /// Use `u64::MAX` to cancel the listing, or the bid price to cancel a bid.
    pub buyer_price: u64,
    pub token_size: u64,
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
    pub trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub listing_config: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) = ah_pda::find_auction_house(&input.authority, &input.treasury_mint);
    let (auctioneer_authority, aa_bump) = pda::find_auctioneer_authority(&auction_house);
    let (ah_auctioneer_pda, _) =
        ah_pda::find_ah_auctioneer_pda(&auction_house, &auctioneer_authority);
    // token_account is always the seller's ATA (the listing's NFT holding account).
    let (token_account, _) = ah_pda::find_ata(&input.seller, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (fee_acc, _) = ah_pda::find_auction_house_fee_account(&auction_house);
    let (trade_state, _) = ah_pda::find_trade_state(
        &wallet_pk,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.buyer_price,
        input.token_size,
    );
    let (listing_config, _) = pda::find_listing_config(
        &input.seller,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );

    let accounts = vec![
        AccountMeta::new_readonly(AUCTION_HOUSE_PROGRAM_ID, false),
        AccountMeta::new(listing_config, false),
        AccountMeta::new_readonly(input.seller, false),
        AccountMeta::new(wallet_pk, true),
        AccountMeta::new(token_account, false),
        AccountMeta::new_readonly(input.token_mint, false),
        AccountMeta::new_readonly(input.authority, false),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new(trade_state, false),
        AccountMeta::new_readonly(auctioneer_authority, false),
        AccountMeta::new_readonly(ah_auctioneer_pda, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
    ];

    let mut args = Vec::with_capacity(17);
    args.push(aa_bump);
    args.extend_from_slice(&input.buyer_price.to_le_bytes());
    args.extend_from_slice(&input.token_size.to_le_bytes());

    let ix = build_auctioneer_instruction(DISC_CANCEL, accounts, args);

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
        trade_state,
        token_account,
        listing_config,
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
            "authority" => pk, "seller" => pk, "token_mint" => pk, "treasury_mint" => pk,
            "buyer_price" => 1000u64, "token_size" => 1u64, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auctioneer_instruction(DISC_CANCEL, vec![], vec![0u8; 17]);
        assert_eq!(ix.program_id, AUCTIONEER_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_CANCEL);
        assert_eq!(ix.data.len(), 25);
    }
}
