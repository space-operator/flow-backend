use super::{
    ATA_PROGRAM_ID, DISC_WITHDRAW, TOKEN_PROGRAM_ID, build_auction_house_instruction,
    payment_account_for, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auction_house_withdraw";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/withdraw.jsonc");

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
    /// Wallet owner of the escrow. One of `wallet` or `authority` must sign.
    pub wallet: Wallet,
    pub authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    pub amount: u64,
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
    pub auction_house: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub receipt_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let (escrow_payment_account, _) = pda::find_escrow_payment_account(&auction_house, &wallet_pk);
    let (auction_house_fee_account, _) = pda::find_auction_house_fee_account(&auction_house);
    let receipt_account = payment_account_for(&wallet_pk, &input.treasury_mint, &TOKEN_PROGRAM_ID);

    let accounts = vec![
        AccountMeta::new_readonly(wallet_pk, true),
        AccountMeta::new(receipt_account, false),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(auction_house_fee_account, false),
        AccountMeta::new(escrow_payment_account, false),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
    ];

    let args_data = input.amount.to_le_bytes().to_vec();
    let ix = build_auction_house_instruction(DISC_WITHDRAW, accounts, args_data);

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
        escrow_payment_account,
        auction_house,
        receipt_account,
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
            "fee_payer" => kp,
            "wallet" => kp,
            "authority" => kp,
            "treasury_mint" => pk,
            "amount" => 1000u64,
            "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix =
            build_auction_house_instruction(DISC_WITHDRAW, vec![], 5u64.to_le_bytes().to_vec());
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_WITHDRAW);
        assert_eq!(ix.data.len(), 16);
    }
}
