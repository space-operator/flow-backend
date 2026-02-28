use crate::attestation_service::calculate_attestation_mint_size;
use crate::prelude::*;

use super::{borsh_encode_attestation_data, pda, to_bytes, to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::CreateTokenizedAttestationBuilder;

const NAME: &str = "create_tokenized_attestation";
const DEFINITION: &str =
    flow_lib::node_definition!("attestation_service/create_tokenized_attestation.jsonc");

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
    pub schema: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub schema_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub recipient: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub nonce: Pubkey,
    /// Schema layout types (e.g., [12, 12, 12, 3] for String, String, String, U64)
    pub layout: JsonValue,
    /// Data values matching the layout (e.g., ["hash", "commitment", "role", 1706892000])
    pub data: JsonValue,
    pub expiry: i64,
    pub name: String,
    pub uri: String,
    pub symbol: String,
    /// Optional: Mint account space in bytes. If 0, None, or not provided, calculated automatically.
    #[serde(default, deserialize_with = "crate::attestation_service::deserialize_optional_u16")]
    pub mint_account_space: u16,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub attestation: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub attestation_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub recipient_token_account: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive attestation PDA from credential, schema, and nonce
    let (attestation, _bump) =
        pda::find_attestation(&input.credential, &input.schema, &input.nonce);
    // Derive attestation_mint PDA from attestation
    let (attestation_mint, _bump) = pda::find_attestation_mint(&attestation);
    // Derive recipient token account (ATA) from recipient and attestation_mint
    let (recipient_token_account, _bump) =
        pda::find_recipient_token_account(&input.recipient, &attestation_mint);
    // Derive SAS authority PDA (program signer)
    let (sas_authority, _bump) = pda::derive_sas_authority_address();

    // Convert human-readable layout and data to Borsh-encoded bytes
    let layout_bytes = to_bytes(&input.layout);
    tracing::info!(
        "create_tokenized_attestation: layout={:?}, layout_bytes={:?}, data={:?}",
        input.layout,
        layout_bytes,
        input.data
    );
    let encoded_data = borsh_encode_attestation_data(&layout_bytes, &input.data);
    tracing::info!(
        "create_tokenized_attestation: encoded_data len={}, bytes={:?}",
        encoded_data.len(),
        encoded_data
    );

    // Calculate mint_account_space if not provided (0 means auto-calculate)
    let mint_account_space = if input.mint_account_space == 0 {
        let calculated = calculate_attestation_mint_size(
            &input.name,
            &input.symbol,
            &input.uri,
            encoded_data.len(),
        );
        tracing::info!(
            "create_tokenized_attestation: auto-calculated mint_account_space={} (name={}, symbol={}, uri_len={}, data_len={})",
            calculated,
            input.name.len(),
            input.symbol.len(),
            input.uri.len(),
            encoded_data.len()
        );
        calculated
    } else {
        tracing::info!(
            "create_tokenized_attestation: using provided mint_account_space={}",
            input.mint_account_space
        );
        input.mint_account_space
    };

    // Build instruction using builder pattern
    let instruction = CreateTokenizedAttestationBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .credential(to_pubkey_v2(&input.credential))
        .schema(to_pubkey_v2(&input.schema))
        .attestation(to_pubkey_v2(&attestation))
        .schema_mint(to_pubkey_v2(&input.schema_mint))
        .attestation_mint(to_pubkey_v2(&attestation_mint))
        .sas_pda(to_pubkey_v2(&sas_authority))
        .recipient_token_account(to_pubkey_v2(&recipient_token_account))
        .recipient(to_pubkey_v2(&input.recipient))
        .nonce(to_pubkey_v2(&input.nonce))
        .data(encoded_data)
        .expiry(input.expiry)
        .name(input.name.clone())
        .uri(input.uri.clone())
        .symbol(input.symbol.clone())
        .mint_account_space(mint_account_space)
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
                "attestation" => attestation,
                "attestation_mint" => attestation_mint,
                "recipient_token_account" => recipient_token_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature, attestation, attestation_mint, recipient_token_account })
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
    /// Required fields: fee_payer, payer, authority, credential, schema, schema_mint, recipient, nonce, layout, data, expiry, name, uri, symbol, mint_account_space
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "credential" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "schema" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "schema_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "recipient" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "nonce" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "layout" => serde_json::json!([12, 12, 12, 3]),  // String, String, String, U64
            "data" => serde_json::json!(["hash", "commitment", "role", 1706892000]),
            "expiry" => 1000i64,
            "name" => "test_name",
            "uri" => "test_uri",
            "symbol" => "test_symbol",
            "mint_account_space" => 82u16,
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test for create_tokenized_attestation using the run() function.
    /// Creates a tokenized attestation (NFT certificate) under an existing tokenized schema.
    /// Returns a transaction signature and the derived attestation, attestation_mint, and recipient_token_account PDAs.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_create_tokenized_attestation_integration() {
        use crate::attestation_service::{create_credential, create_schema, tokenize_schema};
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
            description: "Test schema for tokenized attestation".to_string(),
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
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            schema,
            schema_mint,
            recipient: wallet.pubkey(),
            nonce,
            layout: serde_json::json!([]), // Empty layout matches empty schema
            data: serde_json::json!([]),   // Empty data for empty layout
            expiry: 0,
            name: "Test Attestation NFT".to_string(),
            uri: "https://example.com/attestation".to_string(),
            symbol: "ATST".to_string(),
            mint_account_space: 82,
            submit: true,
        };

        // Expected PDAs
        let (expected_attestation, _) = pda::find_attestation(&credential, &schema, &nonce);
        let (expected_attestation_mint, _) = pda::find_attestation_mint(&expected_attestation);
        let (expected_recipient_token_account, _) =
            pda::find_recipient_token_account(&wallet.pubkey(), &expected_attestation_mint);

        // Call run function
        let result = run(CommandContext::default(), input).await;

        match result {
            Ok(output) => {
                assert!(output.signature.is_some(), "Expected signature");
                assert_eq!(
                    output.attestation, expected_attestation,
                    "Attestation PDA mismatch"
                );
                assert_eq!(
                    output.attestation_mint, expected_attestation_mint,
                    "Attestation mint PDA mismatch"
                );
                assert_eq!(
                    output.recipient_token_account, expected_recipient_token_account,
                    "Recipient token account mismatch"
                );
                println!(
                    "create_tokenized_attestation succeeded: {:?}",
                    output.signature.unwrap()
                );
            }
            Err(e) => {
                panic!("create_tokenized_attestation failed: {:?}", e);
            }
        }
    }
}
