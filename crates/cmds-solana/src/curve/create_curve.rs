use std::str::FromStr;

use anchor_lang::prelude::AccountMeta;
use borsh::BorshSerialize;
use curve_launchpad::state::{BondingCurve, Global};
use flow_lib::{command::prelude::*, solana::Wallet};
use solana_sdk::{instruction::Instruction, system_program, sysvar};
use spl_associated_token_account::get_associated_token_address;

use crate::{curve::CURVE_LAUNCHPAD_PROGRAM_ID, utils::anchor_sighash};

const NAME: &str = "create_curve";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("curve/create_curve.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    mint: Wallet,
    name: String,
    symbol: String,
    uri: String,
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
    let (global, _) =
        Pubkey::find_program_address(&[Global::SEED_PREFIX], &curve_launchpad_program_id);

    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"mint-authority"], &curve_launchpad_program_id);

    let (bonding_curve, _) = Pubkey::find_program_address(
        &[BondingCurve::SEED_PREFIX, input.mint.pubkey().as_ref()],
        &curve_launchpad_program_id,
    );

    let bonding_curve_token_account =
        get_associated_token_address(&bonding_curve, &input.mint.pubkey());

    let (metadata, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            input.mint.pubkey().as_ref(),
        ],
        &mpl_token_metadata::ID,
    );

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &curve_launchpad_program_id);

    let accounts = vec![
        AccountMeta::new(input.mint.pubkey(), true),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(mint_authority, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(bonding_curve_token_account, false),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(metadata, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(curve_launchpad_program_id, false),
    ];

    let data = curve_launchpad::instruction::Create {
        name: input.name,
        symbol: input.symbol,
        uri: input.uri,
    };

    let instruction = Instruction {
        program_id: curve_launchpad_program_id,
        accounts,
        data: (anchor_sighash("create"), data).try_to_vec()?,
    };

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: vec![input.fee_payer, input.mint],
        instructions: [instruction].into(),
    };

    let signature = ctx.execute(instructions, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use cmds_std as _;

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

        let mint_wallet: Wallet = Wallet::Keypair(Keypair::new());

        let input: Input = from_value(
            value::map! {
                "fee_payer" => value::to_value(&wallet).unwrap(),
                "mint" => value::to_value(&mint_wallet).unwrap(),
                "name" => "Curve".to_string(),
                "symbol" => "CURVE".to_string(),
                "uri" => "https://base.spaceoperator.com/storage/v1/object/public/blinks/dab24e10-8534-497f-af02-000825d48f26/8ec8974d-29bb-4493-8bc4-0dee123b111d.json".to_string(),
                // "submit" => true,
            }
            .into(),
        )
        .unwrap();

        let output = run(ctx, input).await.unwrap();

        dbg!(output.signature.unwrap());
    }
}
