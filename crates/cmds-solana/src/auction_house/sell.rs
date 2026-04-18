use super::{DISC_SELL, TOKEN_PROGRAM_ID, build_auction_house_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;

const NAME: &str = "auction_house_sell";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/sell.jsonc");

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
    pub authority: Wallet,
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
    pub seller_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub free_seller_trade_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program_as_signer: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub auction_house: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let (fee_acc, _) = pda::find_auction_house_fee_account(&auction_house);
    let (token_account, _) = pda::find_ata(&wallet_pk, &input.token_mint, &TOKEN_PROGRAM_ID);
    let (metadata, _) = pda::find_metadata(&input.token_mint);
    let (seller_trade_state, _) = pda::find_trade_state(
        &wallet_pk,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.buyer_price,
        input.token_size,
    );
    let (free_seller_trade_state, _) = pda::find_free_trade_state(
        &wallet_pk,
        &auction_house,
        &token_account,
        &input.treasury_mint,
        &input.token_mint,
        input.token_size,
    );
    let (program_as_signer, _) = pda::find_program_as_signer();

    let accounts = vec![
        AccountMeta::new_readonly(wallet_pk, false),
        AccountMeta::new(token_account, false),
        AccountMeta::new_readonly(metadata, false),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new(seller_trade_state, false),
        AccountMeta::new(free_seller_trade_state, false),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(program_as_signer, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let mut args_data = Vec::with_capacity(16);
    args_data.extend_from_slice(&input.buyer_price.to_le_bytes());
    args_data.extend_from_slice(&input.token_size.to_le_bytes());

    let ix = build_auction_house_instruction(DISC_SELL, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.wallet.clone(),
            input.authority.clone(),
        ]
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
        seller_trade_state,
        free_seller_trade_state,
        program_as_signer,
        auction_house,
        token_account,
        metadata,
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
            "fee_payer" => kp, "wallet" => kp, "authority" => kp,
            "token_mint" => pk, "treasury_mint" => pk,
            "buyer_price" => 1000u64, "token_size" => 1u64, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auction_house_instruction(DISC_SELL, vec![], {
            let mut v = Vec::new();
            v.extend_from_slice(&10u64.to_le_bytes());
            v.extend_from_slice(&1u64.to_le_bytes());
            v
        });
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_SELL);
        assert_eq!(ix.data.len(), 24);
    }
}
