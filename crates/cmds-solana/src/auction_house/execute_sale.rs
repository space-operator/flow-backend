use super::{
    ATA_PROGRAM_ID, DISC_EXECUTE_SALE, TOKEN_PROGRAM_ID, build_auction_house_instruction,
    payment_account_for, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auction_house_execute_sale";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/execute_sale.jsonc");

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
    pub authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub buyer: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seller: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
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
    pub escrow_payment_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub auction_house_treasury: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub buyer_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seller_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub free_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program_as_signer: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub auction_house: Pubkey,
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
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let (fee_acc, _) = pda::find_auction_house_fee_account(&auction_house);
    let (treasury, _) = pda::find_auction_house_treasury(&auction_house);
    let (token_account, _) = pda::find_ata(&input.seller, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (metadata, _) = pda::find_metadata(&input.token_mint);
    let seller_payment_receipt_account =
        payment_account_for(&input.seller, &input.treasury_mint, &TOKEN_PROGRAM_ID);
    let (buyer_receipt_token_account, _) =
        pda::find_ata(&input.buyer, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (escrow_payment_account, _) =
        pda::find_escrow_payment_account(&auction_house, &input.buyer);
    let (buyer_trade_state, _) = pda::find_trade_state(
        &input.buyer,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.buyer_price,
        input.token_size,
    );
    let (seller_trade_state, _) = pda::find_trade_state(
        &input.seller,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.buyer_price,
        input.token_size,
    );
    let (free_trade_state, _) = pda::find_free_trade_state(
        &input.seller,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );
    let (program_as_signer, _) = pda::find_program_as_signer();

    let accounts = vec![
        AccountMeta::new(input.buyer, false),
        AccountMeta::new(input.seller, false),
        AccountMeta::new(token_account, false),
        AccountMeta::new_readonly(input.token_mint, false),
        AccountMeta::new_readonly(metadata, false),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new(seller_payment_receipt_account, false),
        AccountMeta::new(buyer_receipt_token_account, false),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new(treasury, false),
        AccountMeta::new(escrow_payment_account, false),
        AccountMeta::new(buyer_trade_state, false),
        AccountMeta::new(seller_trade_state, false),
        AccountMeta::new(free_trade_state, false),
        AccountMeta::new_readonly(program_as_signer, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
    ];

    let mut args_data = Vec::with_capacity(16);
    args_data.extend_from_slice(&input.buyer_price.to_le_bytes());
    args_data.extend_from_slice(&input.token_size.to_le_bytes());

    let ix = build_auction_house_instruction(DISC_EXECUTE_SALE, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.authority.clone()]
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
        escrow_payment_account,
        auction_house_treasury: treasury,
        buyer_trade_state,
        seller_trade_state,
        free_trade_state,
        program_as_signer,
        auction_house,
        token_account,
        metadata,
        seller_payment_receipt_account,
        buyer_receipt_token_account,
    })
}

#[cfg(test)]
mod tests {
    use super::super::AUCTION_HOUSE_PROGRAM_ID;
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
            "fee_payer" => kp, "authority" => kp,
            "buyer" => pk, "seller" => pk,
            "token_mint" => pk, "treasury_mint" => pk,
            "buyer_price" => 1000u64, "token_size" => 1u64, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auction_house_instruction(DISC_EXECUTE_SALE, vec![], {
            let mut v = Vec::new();
            v.extend_from_slice(&1u64.to_le_bytes());
            v.extend_from_slice(&1u64.to_le_bytes());
            v
        });
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_EXECUTE_SALE);
        assert_eq!(ix.data.len(), 24);
    }
}
