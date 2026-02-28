use crate::prelude::*;

use super::{to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::ChangeSchemaDescriptionBuilder;

const NAME: &str = "change_schema_description";
const DEFINITION: &str =
    flow_lib::node_definition!("attestation_service/change_schema_description.jsonc");

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
    pub description: String,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Build instruction using builder pattern
    let instruction = ChangeSchemaDescriptionBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .credential(to_pubkey_v2(&input.credential))
        .schema(to_pubkey_v2(&input.schema))
        .description(input.description.clone())
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
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
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
    /// Required fields: fee_payer, payer, authority, credential, schema, description
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "credential" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "schema" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "description" => "test_description",
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test for change_schema_description using the run() function.
    /// Changes the description of an existing schema.
    /// Returns a transaction signature.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_change_schema_description_integration() {
        use crate::attestation_service::{create_credential, create_schema};
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
            description: "Initial description".to_string(),
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

        // Step 3: Change schema description using run()
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            schema,
            description: "Updated description".to_string(),
            submit: true,
        };

        // Call run function
        let result = run(CommandContext::default(), input).await;

        match result {
            Ok(output) => {
                assert!(output.signature.is_some(), "Expected signature");
                println!(
                    "change_schema_description succeeded: {:?}",
                    output.signature.unwrap()
                );
            }
            Err(e) => {
                panic!("change_schema_description failed: {:?}", e);
            }
        }
    }
}
