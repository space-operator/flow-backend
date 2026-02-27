//! Create secp256k1 signature verification instruction
//!
//! This is an instruction node that creates a Solana secp256k1 verification instruction
//! and optionally submits it as a transaction.

use crate::{
    prelude::*,
    secp256k1::utils::{parse_bytes, vec_to_array},
};
use solana_secp256k1_program::new_secp256k1_instruction_with_signature;

const NAME: &str = "new_secp256k1_instruction";
const DEFINITION: &str = flow_lib::node_definition!("secp256k1/new_secp256k1_instruction.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    /// Transaction fee payer
    pub fee_payer: Wallet,
    /// Original message that was signed
    pub message: JsonValue,
    /// 64-byte ECDSA signature
    pub signature_bytes: JsonValue,
    /// Recovery ID (0 or 1)
    pub recovery_id: u8,
    /// 20-byte Ethereum address to verify against
    pub eth_address: JsonValue,
    /// Submit transaction to the network
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    /// Transaction signature (if submitted)
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    /// The constructed instruction (serialized)
    pub instruction: JsonValue,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Parse message from JSON to Vec<u8>
    let message = parse_bytes(&input.message)?;

    // Parse and validate signature (must be 64 bytes)
    let signature_bytes_vec = parse_bytes(&input.signature_bytes)?;
    let signature_arr: [u8; 64] = vec_to_array(signature_bytes_vec, "signature_bytes")?;

    // Parse and validate Ethereum address (must be 20 bytes)
    let eth_address_vec = parse_bytes(&input.eth_address)?;
    let eth_address: [u8; 20] = vec_to_array(eth_address_vec, "eth_address")?;

    // Create the secp256k1 verification instruction
    let instruction = new_secp256k1_instruction_with_signature(
        &message,
        &signature_arr,
        input.recovery_id,
        &eth_address,
    );

    // Serialize instruction for output
    let instruction_json = serde_json::json!({
        "program_id": instruction.program_id.to_string(),
        "accounts": instruction.accounts.iter().map(|a| {
            serde_json::json!({
                "pubkey": a.pubkey.to_string(),
                "is_signer": a.is_signer,
                "is_writable": a.is_writable,
            })
        }).collect::<Vec<_>>(),
        "data": instruction.data,
    });

    // Build Instructions struct for execution
    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone()].into_iter().collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let tx_signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature: tx_signature,
        instruction: instruction_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "message" => serde_json::json!([0x48, 0x65, 0x6c, 0x6c, 0x6f]),
            "signature_bytes" => serde_json::json!((0..64).collect::<Vec<u8>>()),
            "recovery_id" => 0u8,
            "eth_address" => serde_json::json!((0..20).collect::<Vec<u8>>()),
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
