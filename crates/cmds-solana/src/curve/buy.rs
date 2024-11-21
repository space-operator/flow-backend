use std::str::FromStr;

use anchor_lang::prelude::AccountMeta;
use borsh::BorshSerialize;
use curve_launchpad::state::{BondingCurve, Global};
use flow_lib::{command::prelude::*, solana::Wallet};
use solana_sdk::{instruction::Instruction, system_program};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};

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
    dbg!(&bonding_curve);
    dbg!(&bonding_curve_token_account);
    dbg!(&user_token_account);

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &curve_launchpad_program_id);

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(input.fee_recipient, false),
        AccountMeta::new(input.mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(bonding_curve_token_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(curve_launchpad_program_id, false),
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

    let user_token_account = get_associated_token_address(&input.fee_payer.pubkey(), &input.mint);

    // Check if the token account already exists
    let mut instructions_vec = vec![];
    if ctx
        .solana_client
        .get_account(&user_token_account)
        .await
        .is_err()
    {
        instructions_vec.push(create_associated_token_account(
            &input.fee_payer.pubkey(),
            &input.fee_payer.pubkey(),
            &input.mint,
            &spl_token::id(),
        ));
    }
    instructions_vec.push(instruction);

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: instructions_vec.into(),
    };

    let signature = ctx.execute(instructions, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use flow::{flow_run_events::event_channel, FlowGraph};
    use flow_lib::{config::client::ClientConfig, FlowConfig};

    use value::from_value;

    use super::*;

    #[derive(Deserialize)]
    struct TestFile {
        flow: ClientConfig,
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }
    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        const KEYPAIR: &str =
            "oLXLpXdGn6RjMHz3fvcPdGNUDQxXu91t7YAFbtRew3TFVPHAU1UrZJpgiHDLKDtrWZRQg6trQFFp6zEX2TQ1S3k";

        let wallet = Wallet::Keypair(Keypair::from_base58_string(KEYPAIR));
        dbg!(&wallet.pubkey());

        let fee_recipient =
            Pubkey::from_str("xe7tyibJPS22rBHFrhypM7DwguCJtAc9mHXNRU5CEYG").unwrap();

        let mint = Pubkey::from_str("D9ytTiHkUE5uDwCnVGwekErqgq4VoNyBz31YY7QLZbXm").unwrap();

        let input: Input = from_value(
            value::map! {
                "fee_payer" => value::to_value(&wallet).unwrap(),
                "fee_recipient" => value::to_value(&fee_recipient).unwrap(),
                "mint" => value::to_value(&mint).unwrap(),
                "token_amount" => 1000000,
                "max_sol_cost" => 100,
                "submit" => true,
            }
            .into(),
        )
        .unwrap();
        let output = run(ctx, input).await.unwrap();

        dbg!(output.signature.unwrap());
    }
}
