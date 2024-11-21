use std::str::FromStr;

use curve_launchpad::state::Global;
use flow_lib::{command::prelude::*, solana::Wallet};

use anchor_lang::{prelude::AccountMeta, AccountDeserialize};
use solana_sdk::{instruction::Instruction, system_program};

use crate::{curve::CURVE_LAUNCHPAD_PROGRAM_ID, utils::anchor_sighash};

const NAME: &str = "initialize_curve";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("curve/initialize.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
    // pub authority: Pubkey,
    // pub initialized: bool,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let curve_launchpad_program_id = Pubkey::from_str(CURVE_LAUNCHPAD_PROGRAM_ID).unwrap();

    // Derive PDA for global state
    let (global, _) = Pubkey::find_program_address(&[b"global"], &curve_launchpad_program_id);
    tracing::info!("global: {:?}", global);

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new(global, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id: curve_launchpad_program_id,
        accounts,
        data: anchor_sighash("initialize").into(),
    };

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
    };

    //
    // let account_data = ctx.solana_client.get_account(&global).await?;

    // let global_account: Global =
    //     Global::try_deserialize(&mut &account_data.data[..]).map_err(|e| anyhow::anyhow!(e))?;

    let signature = ctx
        .execute(
            instructions,
            value::map! {
                // "authority" => value::to_value(&global_account.authority).unwrap(),
                // "initialized" => value::to_value(&global_account.initialized).unwrap(),
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use crate::prelude::value::from_value;
    use flow_lib::solana::Wallet;

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

        let input: Input = from_value(
            value::map! {
                "fee_payer" => value::to_value(&wallet).unwrap(),
                "submit" => true,
            }
            .into(),
        )
        .unwrap();

        let output = run(ctx, input).await.unwrap();

        dbg!(output.signature.unwrap());
    }
}
