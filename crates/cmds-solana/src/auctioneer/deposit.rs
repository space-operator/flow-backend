use super::{AUCTIONEER_PROGRAM_ID, DISC_DEPOSIT, build_auctioneer_instruction, pda};
use crate::auction_house::{
    AUCTION_HOUSE_PROGRAM_ID, TOKEN_PROGRAM_ID, payment_account_for, pda as ah_pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;

const NAME: &str = "auctioneer_deposit";
const DEFINITION: &str = flow_lib::node_definition!("auctioneer/deposit.jsonc");

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
    pub auction_house: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub escrow_payment_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) = ah_pda::find_auction_house(&input.authority, &input.treasury_mint);
    let (auctioneer_authority, aa_bump) = pda::find_auctioneer_authority(&auction_house);
    let (ah_auctioneer_pda, _) =
        ah_pda::find_ah_auctioneer_pda(&auction_house, &auctioneer_authority);
    let (escrow_payment_account, esc_bump) =
        ah_pda::find_escrow_payment_account(&auction_house, &wallet_pk);
    let (fee_acc, _) = ah_pda::find_auction_house_fee_account(&auction_house);
    let payment_account = payment_account_for(&wallet_pk, &input.treasury_mint, &TOKEN_PROGRAM_ID);
    let transfer_authority = wallet_pk;

    let accounts = vec![
        AccountMeta::new_readonly(AUCTION_HOUSE_PROGRAM_ID, false),
        AccountMeta::new_readonly(wallet_pk, true),
        AccountMeta::new(payment_account, false),
        AccountMeta::new_readonly(transfer_authority, false),
        AccountMeta::new(escrow_payment_account, false),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new_readonly(input.authority, false),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new_readonly(auctioneer_authority, false),
        AccountMeta::new_readonly(ah_auctioneer_pda, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let mut args = Vec::with_capacity(10);
    args.push(esc_bump);
    args.push(aa_bump);
    args.extend_from_slice(&input.amount.to_le_bytes());

    let ix = build_auctioneer_instruction(DISC_DEPOSIT, accounts, args);

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
        escrow_payment_account,
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
            "authority" => pk, "treasury_mint" => pk,
            "amount" => 1000u64, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auctioneer_instruction(DISC_DEPOSIT, vec![], vec![0u8; 10]);
        assert_eq!(ix.program_id, AUCTIONEER_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_DEPOSIT);
        assert_eq!(ix.data.len(), 18);
    }
}
