use std::str::FromStr;

use anchor_lang::prelude::AccountMeta;
use borsh::BorshSerialize;
use curve_launchpad::state::Global;
use flow_lib::{command::prelude::*, solana::Wallet};
use solana_sdk::{instruction::Instruction, system_program};

use crate::{curve::CURVE_LAUNCHPAD_PROGRAM_ID, utils::anchor_sighash};

const NAME: &str = "set_params";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("curve/set_params.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    fee_recipient: Pubkey,
    #[serde_as(as = "AsPubkey")]
    withdraw_authority: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    initial_token_supply: u64,
    fee_basis_points: u64,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let curve_launchpad_program_id = Pubkey::from_str(CURVE_LAUNCHPAD_PROGRAM_ID).unwrap();

    let (global, _) =
        Pubkey::find_program_address(&[Global::SEED_PREFIX], &curve_launchpad_program_id);

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &curve_launchpad_program_id);

    let accounts = vec![
        AccountMeta::new(global, false),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(curve_launchpad_program_id, false),
    ];

    let data = curve_launchpad::instruction::SetParams {
        fee_recipient: input.fee_recipient,
        withdraw_authority: input.withdraw_authority,
        initial_virtual_token_reserves: input.initial_virtual_token_reserves,
        initial_virtual_sol_reserves: input.initial_virtual_sol_reserves,
        initial_real_token_reserves: input.initial_real_token_reserves,
        fee_basis_points: input.fee_basis_points,
        inital_token_supply: input.initial_token_supply,
    };

    let instruction = Instruction {
        program_id: curve_launchpad_program_id,
        accounts,
        data: (anchor_sighash("set_params"), data).try_to_vec()?,
    };

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
    };

    let signature = ctx.execute(instructions, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {

    use value::from_value;

    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        const KEYPAIR: &str =
            "3LUpzbebV5SCftt8CPmicbKxNtQhtJegEz4n8s6LBf3b1s4yfjLapgJhbMERhP73xLmWEP2XJ2Rz7Y3TFiYgTpXv";
        let wallet = Wallet::Keypair(Keypair::from_base58_string(KEYPAIR));

        let fee_recipient = Wallet::Keypair(Keypair::new()).pubkey();
        let withdraw_authority = Wallet::Keypair(Keypair::new()).pubkey();

        let default_decimals = 6;
        let initial_token_supply = 1_000_000_000 * 10u64.pow(default_decimals);

        let initial_virtual_token_reserves: u64 = 1_073_000_000_000_000;
        let initial_virtual_sol_reserves: u64 = 30_000_000_000;
        let initial_real_token_reserves: u64 = 793_100_000_000_000;
        let fee_basis_points: u64 = 50;

        let test: Vec<u8> = vec![
            95, 95, 101, 118, 101, 110, 116, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121,
        ];
        dbg!(String::from_utf8(test.to_vec()).unwrap());

        let input: Input = from_value(
            value::map! {
                "fee_payer" => value::to_value(&wallet).unwrap(),
                "fee_recipient" => value::to_value(&fee_recipient).unwrap(),
                "withdraw_authority" => value::to_value(&withdraw_authority).unwrap(),
                "initial_virtual_token_reserves" => value::to_value(&initial_virtual_token_reserves).unwrap(),
                "initial_virtual_sol_reserves" => value::to_value(&initial_virtual_sol_reserves).unwrap(),
                "initial_real_token_reserves" => value::to_value(&initial_real_token_reserves).unwrap(),
                "initial_token_supply" => value::to_value(&initial_token_supply).unwrap(),
                "fee_basis_points" => value::to_value(&fee_basis_points).unwrap(),
                "submit" => true,
            }
            .into(),
        )
        .unwrap();
        let output = run(ctx, input).await.unwrap();

        dbg!(output.signature.unwrap());
    }
}
