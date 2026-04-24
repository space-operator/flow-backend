use super::{
    DISC_DEPOSIT, TOKEN_PROGRAM_ID, build_auction_house_instruction, payment_account_for, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;

const NAME: &str = "auction_house_deposit";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/deposit.jsonc");

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
    pub payment_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let (escrow_payment_account, _) = pda::find_escrow_payment_account(&auction_house, &wallet_pk);
    let (auction_house_fee_account, _) = pda::find_auction_house_fee_account(&auction_house);
    let payment_account = payment_account_for(&wallet_pk, &input.treasury_mint, &TOKEN_PROGRAM_ID);
    // For SPL flows the wallet itself approves the transfer (no external delegate).
    let transfer_authority = wallet_pk;

    let accounts = vec![
        AccountMeta::new_readonly(wallet_pk, true),
        AccountMeta::new(payment_account, false),
        AccountMeta::new_readonly(transfer_authority, false),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(escrow_payment_account, false),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new(auction_house_fee_account, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let args_data = input.amount.to_le_bytes().to_vec();
    let ix = build_auction_house_instruction(DISC_DEPOSIT, accounts, args_data);

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
        payment_account,
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
        let ah = Pubkey::new_unique();
        let wallet = Pubkey::new_unique();
        let (esc, _) = pda::find_escrow_payment_account(&ah, &wallet);
        let (fee, _) = pda::find_auction_house_fee_account(&ah);
        let p = Pubkey::new_unique();

        let accounts = vec![
            AccountMeta::new_readonly(wallet, true),
            AccountMeta::new(p, false),
            AccountMeta::new_readonly(p, false),
            AccountMeta::new_readonly(ah, false),
            AccountMeta::new(esc, false),
            AccountMeta::new_readonly(p, false),
            AccountMeta::new_readonly(p, true),
            AccountMeta::new(fee, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ];
        let ix =
            build_auction_house_instruction(DISC_DEPOSIT, accounts, 5u64.to_le_bytes().to_vec());
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 11);
        assert_eq!(ix.data.len(), 16);
        assert_eq!(ix.data[..8], DISC_DEPOSIT);
    }
}
