use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "create_direct_distribution";
const DEFINITION: &str = flow_lib::node_definition!("rewards/create_direct_distribution.jsonc");

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
    pub seeds: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
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
    pub distribution: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub distribution_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (distribution, bump) = pda::find_direct_distribution(
        &input.mint,
        &input.authority.pubkey(),
        &input.seeds.pubkey(),
    );
    let (distribution_vault, _) = pda::find_ata(&distribution, &input.mint, &input.token_program);
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(input.seeds.pubkey(), true),
        AccountMeta::new(distribution, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new(distribution_vault, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data = Vec::with_capacity(10);
    args_data.push(bump);
    args_data.push(input.revocable);
    args_data.extend_from_slice(&input.clawback_ts.to_le_bytes());

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::CreateDirectDistribution,
        accounts,
        args_data,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.authority, input.seeds]
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
        distribution,
        distribution_vault,
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
            "seeds" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "revocable" => 1u8,
            "clawback_ts" => 0i64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_pda_derivation() {
        let authority = Keypair::new();
        let seeds_kp = Keypair::new();
        let mint = Pubkey::new_unique();

        let (dist1, b1) =
            pda::find_direct_distribution(&mint, &authority.pubkey(), &seeds_kp.pubkey());
        let (dist2, b2) =
            pda::find_direct_distribution(&mint, &authority.pubkey(), &seeds_kp.pubkey());
        assert_eq!(dist1, dist2);
        assert_eq!(b1, b2);
        assert_ne!(dist1, Pubkey::default());

        let (vault, _) = pda::find_ata(&dist1, &mint, &super::default_token_program());
        assert_ne!(vault, Pubkey::default());
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_create_direct_distribution() {
        tracing_subscriber::fmt::try_init().ok();

        let wallet = crate::test_utils::test_wallet();
        let ctx = crate::test_utils::test_context();

        crate::test_utils::ensure_funded(ctx.solana_client(), &wallet.pubkey(), 0.1).await;

        let seeds: Wallet = Keypair::new().into();
        // Use native SOL wrapped mint for devnet testing
        let mint = solana_program::pubkey!("So11111111111111111111111111111111111111112");

        let output = run(
            ctx,
            Input {
                payer: wallet.clone(),
                authority: wallet.clone(),
                seeds,
                mint,
                token_program: super::default_token_program(),
                revocable: 1,
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
        assert_ne!(output.distribution, Pubkey::default());
        assert_ne!(output.distribution_vault, Pubkey::default());
    }
}
