use std::str::FromStr;

use anchor_lang::prelude::AccountMeta;
use borsh::BorshSerialize;
use curve_launchpad::state::{BondingCurve, Global};
use flow_lib::{command::prelude::*, solana::Wallet};
use solana_sdk::{instruction::Instruction, system_program};
use spl_associated_token_account::get_associated_token_address;

use crate::{curve::CURVE_LAUNCHPAD_PROGRAM_ID, utils::anchor_sighash};

const NAME: &str = "buy";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("curve/buy.json");
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
    mint: Pubkey,
    token_amount: u64,
    max_sol_cost: u64,
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

    // Derive PDAs
    let (bonding_curve, _) = Pubkey::find_program_address(
        &[BondingCurve::SEED_PREFIX, input.mint.as_ref()],
        &curve_launchpad_program_id,
    );
    let (global, _) =
        Pubkey::find_program_address(&[Global::SEED_PREFIX], &curve_launchpad_program_id);

    let user_token_account = get_associated_token_address(&input.fee_payer.pubkey(), &input.mint);

    let bonding_curve_token_account = get_associated_token_address(&bonding_curve, &input.mint);

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(input.fee_recipient, true),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(bonding_curve_token_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    let data = curve_launchpad::instruction::Buy {
        token_amount: input.token_amount,
        max_sol_cost: input.max_sol_cost,
    };

    let instruction = Instruction {
        program_id: curve_launchpad_program_id,
        accounts,
        data: (anchor_sighash("buy"), data).try_to_vec()?,
    };

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
    };

    let signature = ctx.execute(instructions, value::map! {}).await?.signature;

    Ok(Output { signature })
}

// #[cfg(test)]
// mod tests {
//     use flow::{flow_run_events::event_channel, FlowGraph};
//     use flow_lib::{config::client::ClientConfig, FlowConfig};

//     use value::from_value;

//     use super::*;

//     #[derive(Deserialize)]
//     struct TestFile {
//         flow: ClientConfig,
//     }

//     #[test]
//     fn test_build() {
//         build().unwrap();
//     }
//     #[tokio::test]
//     async fn test_run() {
//         let ctx = Context::default();

//         const KEYPAIR: &str =
//             "3LUpzbebV5SCftt8CPmicbKxNtQhtJegEz4n8s6LBf3b1s4yfjLapgJhbMERhP73xLmWEP2XJ2Rz7Y3TFiYgTpXv";
//         let wallet = Wallet::Keypair(Keypair::from_base58_string(KEYPAIR));
//         dbg!(&wallet.pubkey());

//         let input: Input = from_value(
//             value::map! {
//                 "fee_payer" => value::to_value(&wallet).unwrap(),
//                 "mint" => value::to_value(&mint_str).unwrap(),
//                 "name" => "Curve".to_string(),
//                 "symbol" => "CURVE".to_string(),
//                 "uri" => "https://base.spaceoperator.com/storage/v1/object/public/blinks/dab24e10-8534-497f-af02-000825d48f26/8ec8974d-29bb-4493-8bc4-0dee123b111d.json".to_string(),
//                 "submit" => true,
//             }
//             .into(),
//         )
//         .unwrap();
//         let output = run(ctx, input).await.unwrap();

//         dbg!(output.signature.unwrap());
//     }
// }
