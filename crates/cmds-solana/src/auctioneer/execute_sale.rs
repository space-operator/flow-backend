use super::{
    AUCTIONEER_BUYER_PRICE, AUCTIONEER_PROGRAM_ID, DISC_EXECUTE_SALE, build_auctioneer_instruction,
    pda,
};
use crate::auction_house::{
    ATA_PROGRAM_ID, AUCTION_HOUSE_PROGRAM_ID, TOKEN_PROGRAM_ID, payment_account_for, pda as ah_pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;

const NAME: &str = "auctioneer_execute_sale";
const DEFINITION: &str = flow_lib::node_definition!("auctioneer/execute_sale.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub buyer: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seller: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    /// The winning bid price — must match the `buyer_trade_state` seeds.
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
    pub listing_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub buyer_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seller_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub free_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub escrow_payment_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub metadata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seller_payment_receipt_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub buyer_receipt_token_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (auction_house, _) = ah_pda::find_auction_house(&input.authority, &input.treasury_mint);
    let (auctioneer_authority, aa_bump) = pda::find_auctioneer_authority(&auction_house);
    let (ah_auctioneer_pda, _) =
        ah_pda::find_ah_auctioneer_pda(&auction_house, &auctioneer_authority);
    let (token_account, _) = ah_pda::find_ata(&input.seller, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (metadata, _) = ah_pda::find_metadata(&input.token_mint);
    let (fee_acc, _) = ah_pda::find_auction_house_fee_account(&auction_house);
    let (treasury, _) = ah_pda::find_auction_house_treasury(&auction_house);
    let (escrow_payment_account, esc_bump) =
        ah_pda::find_escrow_payment_account(&auction_house, &input.buyer);
    let seller_payment_receipt_account =
        payment_account_for(&input.seller, &input.treasury_mint, &TOKEN_PROGRAM_ID);
    let (buyer_receipt_token_account, _) =
        ah_pda::find_ata(&input.buyer, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (buyer_trade_state, _) = ah_pda::find_trade_state(
        &input.buyer,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.buyer_price,
        input.token_size,
    );
    let (seller_trade_state, _) = ah_pda::find_trade_state(
        &input.seller,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        AUCTIONEER_BUYER_PRICE,
        input.token_size,
    );
    let (free_trade_state, fts_bump) = ah_pda::find_free_trade_state(
        &input.seller,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );
    let (program_as_signer, pas_bump) = ah_pda::find_program_as_signer();
    let (listing_config, _) = pda::find_listing_config(
        &input.seller,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );

    // See auctioneer/deposit.rs for the AH sign-off rationale.
    let authority_is_signer = input.authority == input.fee_payer.pubkey();

    let accounts = vec![
        AccountMeta::new_readonly(AUCTION_HOUSE_PROGRAM_ID, false),
        AccountMeta::new(listing_config, false),
        AccountMeta::new(input.buyer, false),
        AccountMeta::new(input.seller, false),
        AccountMeta::new(token_account, false),
        AccountMeta::new_readonly(input.token_mint, false),
        AccountMeta::new_readonly(metadata, false),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new(escrow_payment_account, false),
        AccountMeta::new(seller_payment_receipt_account, false),
        AccountMeta::new(buyer_receipt_token_account, false),
        AccountMeta::new_readonly(input.authority, authority_is_signer),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new(treasury, false),
        AccountMeta::new(buyer_trade_state, false),
        AccountMeta::new(seller_trade_state, false),
        AccountMeta::new(free_trade_state, false),
        AccountMeta::new_readonly(auctioneer_authority, false),
        AccountMeta::new_readonly(ah_auctioneer_pda, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
        AccountMeta::new_readonly(program_as_signer, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let mut args = Vec::with_capacity(20);
    args.push(esc_bump);
    args.push(fts_bump);
    args.push(pas_bump);
    args.push(aa_bump);
    args.extend_from_slice(&input.buyer_price.to_le_bytes());
    args.extend_from_slice(&input.token_size.to_le_bytes());

    let ix = build_auctioneer_instruction(DISC_EXECUTE_SALE, accounts, args);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone()].into_iter().collect(),
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
        buyer_trade_state,
        seller_trade_state,
        free_trade_state,
        escrow_payment_account,
        token_account,
        metadata,
        seller_payment_receipt_account,
        buyer_receipt_token_account,
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
            "fee_payer" => kp,
            "authority" => pk, "buyer" => pk, "seller" => pk,
            "token_mint" => pk, "treasury_mint" => pk,
            "buyer_price" => 1000u64, "token_size" => 1u64, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auctioneer_instruction(DISC_EXECUTE_SALE, vec![], vec![0u8; 20]);
        assert_eq!(ix.program_id, AUCTIONEER_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_EXECUTE_SALE);
        assert_eq!(ix.data.len(), 28);
    }
}
