use super::{
    BalanceSource, REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction,
    default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "create_continuous_pool";
const DEFINITION: &str = flow_lib::node_definition!("rewards/create_continuous_pool.jsonc");

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
    pub payer: Wallet,
    pub authority: Wallet,
    pub seed: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub tracked_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub reward_token_program: Pubkey,
    pub balance_source: JsonValue,
    pub revocable: u8,
    pub clawback_ts: i64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub reward_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (reward_pool, bump) = pda::find_reward_pool(
        &input.reward_mint,
        &input.tracked_mint,
        &input.authority.pubkey(),
        &input.seed.pubkey(),
    );
    let (reward_vault, _) = pda::find_ata(
        &reward_pool,
        &input.reward_mint,
        &input.reward_token_program,
    );
    let (event_authority, _) = pda::find_event_authority();

    let balance_source: BalanceSource = serde_json::from_value(input.balance_source)?;

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(input.seed.pubkey(), true),
        AccountMeta::new(reward_pool, false),
        AccountMeta::new_readonly(input.tracked_mint, false),
        AccountMeta::new_readonly(input.reward_mint, false),
        AccountMeta::new(reward_vault, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(input.reward_token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data: Vec<u8> = Vec::new();
    args_data.push(bump);
    args_data.extend_from_slice(&borsh::to_vec(&balance_source)?);
    args_data.push(input.revocable);
    args_data.extend_from_slice(&input.clawback_ts.to_le_bytes());

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::CreateContinuousPool,
        accounts,
        args_data,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.authority, input.seed]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        reward_pool,
        reward_vault,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_keypair::Keypair;
    use solana_signer::Signer;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        let input = value::map! {
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "seed" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "tracked_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "balance_source" => serde_json::json!("OnChain"),
            "revocable" => 0u8,
            "clawback_ts" => 0i64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_pda_derivation() {
        let authority = Keypair::new();
        let seed_kp = Keypair::new();
        let tracked_mint = Pubkey::new_unique();
        let reward_mint = Pubkey::new_unique();

        let (pool1, b1) = pda::find_reward_pool(
            &reward_mint,
            &tracked_mint,
            &authority.pubkey(),
            &seed_kp.pubkey(),
        );
        let (pool2, b2) = pda::find_reward_pool(
            &reward_mint,
            &tracked_mint,
            &authority.pubkey(),
            &seed_kp.pubkey(),
        );
        assert_eq!(pool1, pool2);
        assert_eq!(b1, b2);
        assert_ne!(pool1, Pubkey::default());

        let (vault, _) = pda::find_ata(&pool1, &reward_mint, &super::default_token_program());
        assert_ne!(vault, Pubkey::default());
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_create_continuous_pool() {
        tracing_subscriber::fmt::try_init().ok();

        let wallet = crate::test_utils::test_wallet();
        let ctx = crate::test_utils::test_context();

        crate::test_utils::ensure_funded(ctx.solana_client(), &wallet.pubkey(), 0.1).await;

        let seed: Wallet = Keypair::new().into();
        let sol_mint = solana_program::pubkey!("So11111111111111111111111111111111111111112");

        let output = run(
            ctx,
            Input {
                payer: wallet.clone(),
                authority: wallet.clone(),
                seed,
                tracked_mint: sol_mint,
                reward_mint: sol_mint,
                reward_token_program: super::default_token_program(),
                balance_source: serde_json::json!("OnChain"),
                revocable: 0,
                clawback_ts: 0,
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(&output.signature);
        assert!(
            output.signature.is_some(),
            "expected a transaction signature"
        );
        assert_ne!(output.reward_pool, Pubkey::default());
        assert_ne!(output.reward_vault, Pubkey::default());
    }
}
