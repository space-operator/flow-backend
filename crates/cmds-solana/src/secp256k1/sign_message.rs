//! Sign a message using secp256k1 ECDSA
//!
//! This is a utility node that performs pure computation - no transaction submission.

use solana_secp256k1_program::sign_message;

use crate::{
    prelude::*,
    secp256k1::utils::{parse_bytes, vec_to_array},
};

const NAME: &str = "sign_message";
const DEFINITION: &str = flow_lib::node_definition!("secp256k1/sign_message.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    /// 32-byte secp256k1 private key
    pub private_key: JsonValue,
    /// Message bytes to sign
    pub message: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    /// 64-byte ECDSA signature as byte array
    pub signature: JsonValue,
    /// Recovery ID (0 or 1)
    pub recovery_id: u8,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Parse private key from JSON to Vec<u8>
    let private_key_bytes = parse_bytes(&input.private_key)?;
    let private_key: [u8; 32] = vec_to_array(private_key_bytes, "private_key")?;

    // Parse message from JSON to Vec<u8>
    let message = parse_bytes(&input.message)?;

    // Call the secp256k1 sign_message function
    let (signature_arr, recovery_id) = sign_message(&private_key, &message)
        .map_err(|e| anyhow::anyhow!("signing failed: {:?}", e))?;

    // Convert signature to JSON array
    let signature = serde_json::json!(signature_arr.to_vec());

    Ok(Output {
        signature,
        recovery_id,
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
    async fn test_sign_message() {
        // Test private key (32 bytes) - deterministic test key
        let private_key = serde_json::json!([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f
        ]);

        let message = serde_json::json!([0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"

        let input = Input {
            private_key,
            message,
        };

        let ctx = CommandContext::default();
        let result = run(ctx, input).await;

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let output = result.unwrap();

        // Verify signature length (should be 64 bytes as JSON array)
        let sig_array = output.signature.as_array().unwrap();
        assert_eq!(sig_array.len(), 64, "Signature should be 64 bytes");

        // Recovery ID should be 0 or 1
        assert!(output.recovery_id <= 1, "Recovery ID should be 0 or 1");
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "private_key" => serde_json::json!(vec![0u8; 32]),
            "message" => serde_json::json!([1, 2, 3]),
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
