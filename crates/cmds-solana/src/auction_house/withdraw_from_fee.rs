use super::{DISC_WITHDRAW_FROM_FEE, build_auction_house_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auction_house_withdraw_from_fee";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/withdraw_from_fee.jsonc");

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
    pub treasury_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub fee_withdrawal_destination: Pubkey,
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
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let (fee_acc, _) = pda::find_auction_house_fee_account(&auction_house);

    let accounts = vec![
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new(input.fee_withdrawal_destination, false),
        AccountMeta::new(auction_house, false),
        AccountMeta::new(fee_acc, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    let ix = build_auction_house_instruction(
        DISC_WITHDRAW_FROM_FEE,
        accounts,
        input.amount.to_le_bytes().to_vec(),
    );

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
        auction_house,
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
            "treasury_mint" => pk, "fee_withdrawal_destination" => pk,
            "amount" => 5u64, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auction_house_instruction(
            DISC_WITHDRAW_FROM_FEE,
            vec![],
            5u64.to_le_bytes().to_vec(),
        );
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_WITHDRAW_FROM_FEE);
        assert_eq!(ix.data.len(), 16);
    }
}
