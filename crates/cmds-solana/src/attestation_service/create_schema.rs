use crate::prelude::*;

use super::{pda, parse_strings, to_bytes, to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::CreateSchemaBuilder;

const NAME: &str = "create_schema";
const DEFINITION: &str = flow_lib::node_definition!("attestation_service/create_schema.jsonc");

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
    pub name: String,
    pub description: String,
    pub layout: JsonValue,
    pub field_names: JsonValue,
    #[serde(default = "default_version")]
    pub version: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

fn default_version() -> u8 {
    1
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub schema: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive schema PDA from credential, name, and version
    let (schema, _bump) = pda::find_schema(&input.credential, &input.name, input.version);

    // Log inputs for debugging
    let layout_bytes = to_bytes(&input.layout);
    tracing::info!(
        "create_schema: name={}, layout={:?}, layout_bytes={:?}, field_names={:?}",
        input.name,
        input.layout,
        layout_bytes,
        input.field_names
    );

    // Build instruction using builder pattern
    let instruction = CreateSchemaBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .credential(to_pubkey_v2(&input.credential))
        .schema(to_pubkey_v2(&schema))
        .name(input.name.clone())
        .description(input.description.clone())
        .layout(layout_bytes)
        .field_names(parse_strings(&input.field_names))
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
                "schema" => schema,
            },
        )
        .await?
        .signature;

    Ok(Output { signature, schema })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: fee_payer, payer, authority, credential, name, description, layout, field_names
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "credential" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "name" => "test_name",
            "description" => "test_description",
            "layout" => serde_json::json!({}),
            "field_names" => serde_json::json!({}),
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test for create_schema using the run() function.
    /// Creates a schema under an existing credential using proper node execution.
    /// Returns a transaction signature and the derived schema PDA.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_create_schema_integration() {
        use crate::attestation_service::create_credential;
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

        // Step 1: Create credential first using run()
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
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            name: schema_name.clone(),
            description: "Test schema for integration testing".to_string(),
            layout: serde_json::json!([]),
            field_names: serde_json::json!([]),
            version: 1,
            submit: true,
        };

        // Expected schema PDA
        let (expected_schema, _) = pda::find_schema(&credential, &schema_name, 1);

        // Call run function
        let result = run(CommandContext::default(), input).await;

        match result {
            Ok(output) => {
                assert!(output.signature.is_some(), "Expected signature");
                assert_eq!(output.schema, expected_schema, "Schema PDA mismatch");
                println!("create_schema succeeded: {:?}", output.signature.unwrap());
            }
            Err(e) => {
                panic!("create_schema failed: {:?}", e);
            }
        }
    }
}
