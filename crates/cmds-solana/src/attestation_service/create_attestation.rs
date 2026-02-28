use crate::prelude::*;

use super::{borsh_encode_attestation_data, pda, to_bytes, to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::CreateAttestationBuilder;

const NAME: &str = "create_attestation";
const DEFINITION: &str = flow_lib::node_definition!("attestation_service/create_attestation.jsonc");

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
    pub nonce: Pubkey,
    /// Schema layout types (e.g., [12, 12, 12, 3] for String, String, String, U64)
    #[serde(default)]
    pub layout: JsonValue,
    /// Data values matching the layout (e.g., ["hash", "commitment", "role", 1706892000])
    pub data: JsonValue,
    pub expiry: i64,
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
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive attestation PDA from credential, schema, and nonce
    let (attestation, _bump) =
        pda::find_attestation(&input.credential, &input.schema, &input.nonce);

    // Parse layout and Borsh-encode data according to schema types
    let layout = to_bytes(&input.layout);
    let encoded_data = borsh_encode_attestation_data(&layout, &input.data);

    // Build instruction using builder pattern
    let instruction = CreateAttestationBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .credential(to_pubkey_v2(&input.credential))
        .schema(to_pubkey_v2(&input.schema))
        .attestation(to_pubkey_v2(&attestation))
        .nonce(to_pubkey_v2(&input.nonce))
        .data(encoded_data)
        .expiry(input.expiry)
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
            },
        )
        .await?
        .signature;

    Ok(Output { signature, attestation })
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

    /// Integration test for create_attestation using the run() function.
    /// Creates an attestation under an existing credential and schema.
    /// Returns a transaction signature and the derived attestation PDA.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_create_attestation_integration() {
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

        // Step 3: Create attestation using run()
        let nonce = Keypair::new().pubkey();
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            schema,
            nonce,
            layout: serde_json::json!([]), // Empty layout matches empty schema
            data: serde_json::json!([]),   // Empty data for empty layout
            expiry: 0,
            submit: true,
        };

        // Expected attestation PDA
        let (expected_attestation, _) = pda::find_attestation(&credential, &schema, &nonce);

        // Call run function
        let result = run(CommandContext::default(), input).await;

        match result {
            Ok(output) => {
                assert!(output.signature.is_some(), "Expected signature");
                assert_eq!(
                    output.attestation, expected_attestation,
                    "Attestation PDA mismatch"
                );
                println!(
                    "create_attestation succeeded: {:?}",
                    output.signature.unwrap()
                );
            }
            Err(e) => {
                panic!("create_attestation failed: {:?}", e);
            }
        }
    }
}
