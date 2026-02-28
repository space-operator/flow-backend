use crate::prelude::*;

use super::{pda, to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::CreateCredentialBuilder;

const NAME: &str = "create_credential";
const DEFINITION: &str = flow_lib::node_definition!("attestation_service/create_credential.jsonc");

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
    pub name: String,
    pub signers: Vec<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub credential: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive credential PDA from authority and name
    let (credential, _bump) = pda::find_credential(&input.authority.pubkey(), &input.name);

    tracing::info!(
        "create_credential: authority={}, name={}, credential_pda={}, signers={:?}",
        input.authority.pubkey(),
        input.name,
        credential,
        input.signers
    );

    // Build instruction using builder pattern
    let instruction = CreateCredentialBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .credential(to_pubkey_v2(&credential))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .name(input.name.clone())
        .signers(input.signers.iter().map(to_pubkey_v2).collect())
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
                "credential" => credential,
            },
        )
        .await?
        .signature;

    Ok(Output { signature, credential })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Integration test for create_credential using the run() function.
    /// Creates a credential account for the authority to manage schemas and attestations.
    /// Returns a transaction signature and the derived credential PDA.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_create_credential_integration() {
        use solana_keypair::Keypair;

        // Setup wallet from test keypair
        let wallet: Wallet = Keypair::from_base58_string(
            "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
        ).into();

        // Generate unique credential name
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let credential_name = format!("TestCred_{}", timestamp);

        // Build Input struct
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            name: credential_name.clone(),
            signers: vec![wallet.pubkey()],
            submit: true,
        };

        // Expected credential PDA
        let (expected_credential, _) = pda::find_credential(&wallet.pubkey(), &credential_name);

        // Call run function
        let result = run(CommandContext::default(), input).await;

        match result {
            Ok(output) => {
                assert!(output.signature.is_some(), "Expected signature");
                assert_eq!(
                    output.credential, expected_credential,
                    "Credential PDA mismatch"
                );
                println!(
                    "create_credential succeeded: {:?}",
                    output.signature.unwrap()
                );
            }
            Err(e) => {
                panic!("create_credential failed: {:?}", e);
            }
        }
    }
}
