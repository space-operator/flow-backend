use crate::prelude::*;

use super::{pda, to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::CloseTokenizedAttestationBuilder;

const NAME: &str = "close_tokenized_attestation";
const DEFINITION: &str =
    flow_lib::node_definition!("attestation_service/close_tokenized_attestation.jsonc");

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
    pub payer: Wallet,
    pub authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub credential: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub attestation: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub attestation_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub attestation_token_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub event_authority: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive event_authority PDA
    let (event_authority, _bump) = pda::find_event_authority();
    // Get attestation program ID
    let attestation_program = pda::attestation_service_program_id();

    // Build instruction using builder pattern
    let instruction = CloseTokenizedAttestationBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .credential(to_pubkey_v2(&input.credential))
        .attestation(to_pubkey_v2(&input.attestation))
        .event_authority(to_pubkey_v2(&event_authority))
        .attestation_program(to_pubkey_v2(&attestation_program))
        .attestation_mint(to_pubkey_v2(&input.attestation_mint))
        .attestation_token_account(to_pubkey_v2(&input.attestation_token_account))
        .instruction();
    let instruction = to_instruction_v3(instruction);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.payer.clone(),
            input.authority.clone(),
        ]
        .into_iter()
        .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx
        .execute(
            ins,
            value::map! {
                "event_authority" => event_authority,
            },
        )
        .await?
        .signature;

    Ok(Output { signature, event_authority })
}

#[cfg(test)]
mod tests {
    use solana_keypair::Signer;

    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: fee_payer, payer, authority, credential, attestation, attestation_mint, attestation_token_account
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "credential" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "attestation" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "attestation_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "attestation_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test for close_tokenized_attestation using the run() function.
    /// Closes (revokes) an existing tokenized attestation and burns the NFT.
    /// Returns a transaction signature and the derived event_authority PDA.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_close_tokenized_attestation_integration() {
        use crate::attestation_service::{
            create_credential, create_schema, create_tokenized_attestation, tokenize_schema,
        };
        use solana_keypair::Keypair;

        // Setup wallet from test keypair
        let wallet: Wallet = Keypair::from_base58_string(
            "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
        ).into();

        // Generate unique names
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let credential_name = format!("TestCred_{}", timestamp);
        let schema_name = format!("TestSchema_{}", timestamp);

        // Step 1: Create credential using run()
        let cred_input = create_credential::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            name: credential_name.clone(),
            signers: vec![wallet.pubkey()],
            submit: true,
        };
        let cred_result = create_credential::run(CommandContext::default(), cred_input).await;
        assert!(
            cred_result.is_ok(),
            "create_credential failed: {:?}",
            cred_result.err()
        );
        let credential = cred_result.unwrap().credential;

        // Step 2: Create schema using run()
        let schema_input = create_schema::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            name: schema_name.clone(),
            description: "Test schema".to_string(),
            layout: serde_json::json!([]),
            field_names: serde_json::json!([]),
            version: 1,
            submit: true,
        };
        let schema_result = create_schema::run(CommandContext::default(), schema_input).await;
        assert!(
            schema_result.is_ok(),
            "create_schema failed: {:?}",
            schema_result.err()
        );
        let schema = schema_result.unwrap().schema;

        // Step 3: Tokenize schema using run()
        let tokenize_input = tokenize_schema::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            schema,
            submit: true,
        };
        let tokenize_result = tokenize_schema::run(CommandContext::default(), tokenize_input).await;
        assert!(
            tokenize_result.is_ok(),
            "tokenize_schema failed: {:?}",
            tokenize_result.err()
        );
        let schema_mint = tokenize_result.unwrap().schema_mint;

        // Step 4: Create tokenized attestation using run()
        let nonce = Keypair::new().pubkey();
        let tokenized_input = create_tokenized_attestation::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            schema,
            schema_mint,
            recipient: wallet.pubkey(),
            nonce,
            layout: serde_json::json!([]),
            data: serde_json::json!([]),
            expiry: 0,
            name: "Test Attestation NFT".to_string(),
            uri: "https://example.com/attestation".to_string(),
            symbol: "ATST".to_string(),
            mint_account_space: 82,
            submit: true,
        };
        let tokenized_result =
            create_tokenized_attestation::run(CommandContext::default(), tokenized_input).await;
        assert!(
            tokenized_result.is_ok(),
            "create_tokenized_attestation failed: {:?}",
            tokenized_result.err()
        );
        let tokenized_output = tokenized_result.unwrap();
        let attestation = tokenized_output.attestation;
        let attestation_mint = tokenized_output.attestation_mint;
        let attestation_token_account = tokenized_output.recipient_token_account;

        // Step 5: Close tokenized attestation using run()
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            attestation,
            attestation_mint,
            attestation_token_account,
            submit: true,
        };

        // Expected event_authority PDA
        let (expected_event_authority, _) = pda::find_event_authority();

        // Call run function
        let result = run(CommandContext::default(), input).await;

        match result {
            Ok(output) => {
                assert!(output.signature.is_some(), "Expected signature");
                assert_eq!(
                    output.event_authority, expected_event_authority,
                    "Event authority PDA mismatch"
                );
                println!(
                    "close_tokenized_attestation succeeded: {:?}",
                    output.signature.unwrap()
                );
            }
            Err(e) => {
                panic!("close_tokenized_attestation failed: {:?}", e);
            }
        }
    }
}
