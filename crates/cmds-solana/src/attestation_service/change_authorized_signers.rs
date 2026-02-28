use crate::prelude::*;

use super::{parse_pubkeys_v2, to_instruction_v3, to_pubkey_v2};
use solana_attestation_service::instructions::ChangeAuthorizedSignersBuilder;

const NAME: &str = "change_authorized_signers";
const DEFINITION: &str =
    flow_lib::node_definition!("attestation_service/change_authorized_signers.jsonc");

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
    pub signers: JsonValue,
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
    let instruction = ChangeAuthorizedSignersBuilder::new()
        .payer(to_pubkey_v2(&input.payer.pubkey()))
        .authority(to_pubkey_v2(&input.authority.pubkey()))
        .credential(to_pubkey_v2(&input.credential))
        .signers(parse_pubkeys_v2(&input.signers))
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
    use solana_keypair::Signer;

    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: fee_payer, payer, authority, credential, signers
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "credential" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "signers" => serde_json::json!({}),
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test for change_authorized_signers using the run() function.
    /// Changes the authorized signers list for an existing credential.
    /// Returns a transaction signature.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_change_authorized_signers_integration() {
        use solana_keypair::Keypair;

        // Setup wallet from test keypair
        let wallet: Wallet = Keypair::from_base58_string(
            "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
        ).into();

        let credential = Keypair::new().pubkey();

        // Change authorized signers using run()
        let input = super::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            signers: serde_json::json!([wallet.pubkey().to_string()]),
            submit: true,
        };

        let output = run(CommandContext::default(), input)
            .await
            .expect("change_authorized_signers failed");

        let signature = output.signature.expect("Expected signature");
        println!("change_authorized_signers succeeded: {:?}", signature);
    }
}
