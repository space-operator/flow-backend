use super::{
    CP_AMM_PROGRAM_ID, POSITION_NFT_ACCOUNT_PREFIX, TOKEN_2022_PROGRAM_ID, anchor_discriminator,
    derive_event_authority, derive_pool_authority, derive_position,
};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "close_position";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/close_position.jsonc");

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
    pub owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub rent_receiver: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let position = derive_position(&input.pool, &input.position_nft_mint);
    let position_nft_account = Pubkey::find_program_address(
        &[
            POSITION_NFT_ACCOUNT_PREFIX,
            input.position_nft_mint.as_ref(),
        ],
        &CP_AMM_PROGRAM_ID,
    )
    .0;
    let event_authority = derive_event_authority();

    let pool_authority = derive_pool_authority();

    let accounts = vec![
        AccountMeta::new(input.position_nft_mint, false), // [0] position_nft_mint (writable)
        AccountMeta::new(position_nft_account, false),    // [1] position_nft_account (writable)
        AccountMeta::new(input.pool, false),              // [2] pool (writable)
        AccountMeta::new(position, false),                // [3] position (writable, close)
        AccountMeta::new_readonly(pool_authority, false), // [4] pool_authority
        AccountMeta::new(input.rent_receiver, false),     // [5] rent_receiver (writable)
        AccountMeta::new_readonly(input.owner.pubkey(), true), // [6] owner (signer)
        AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false), // [7] token_program (Token-2022)
        AccountMeta::new_readonly(event_authority, false), // [8] event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false), // [9] program
    ];

    let data = anchor_discriminator(NAME).to_vec();

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.owner.pubkey(),
        signers: [input.owner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output {
        signature,
        position,
        position_nft_account,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_signer::Signer;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: owner, position_nft_mint, pool, rent_receiver
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "position_nft_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "rent_receiver" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test: constructs Input and calls run().
    /// Requires a funded wallet and network access to pass.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run() {
        use solana_keypair::Keypair;

        let input = Input {
            owner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            rent_receiver: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
