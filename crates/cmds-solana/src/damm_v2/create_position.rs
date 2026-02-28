use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, ATA_PROGRAM_ID, anchor_discriminator, derive_position, derive_event_authority, POSITION_NFT_ACCOUNT_PREFIX};

const NAME: &str = "create_position";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/create_position.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    pub position_nft_mint: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
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
    let position_nft_mint_pubkey = input.position_nft_mint.pubkey();
    let position = derive_position(&input.pool, &position_nft_mint_pubkey);
    let position_nft_account = Pubkey::find_program_address(
        &[POSITION_NFT_ACCOUNT_PREFIX, position_nft_mint_pubkey.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0;
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),              // payer (writable signer)
        AccountMeta::new_readonly(input.owner, false),             // owner
        AccountMeta::new(input.pool, false),                       // pool (writable)
        AccountMeta::new(position, false),                         // position (writable, init)
        AccountMeta::new(position_nft_mint_pubkey, true),          // position_nft_mint (writable signer)
        AccountMeta::new(position_nft_account, false),             // position_nft_account (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),        // token_program
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),          // associated_token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),       // system_program
        AccountMeta::new_readonly(event_authority, false),         // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false),       // program
    ];

    let data = anchor_discriminator(NAME).to_vec();

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.position_nft_mint].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, position, position_nft_account })
}

#[cfg(test)]
mod tests {
    use solana_signer::Signer;
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: payer, owner, position_nft_mint, pool
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "owner" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_nft_mint" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
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
            payer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            owner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
