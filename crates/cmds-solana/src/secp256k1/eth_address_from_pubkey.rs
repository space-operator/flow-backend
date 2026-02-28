//! Derive Ethereum address from secp256k1 public key
//!
//! This is a utility node that performs pure computation - no transaction submission.

use crate::{
    prelude::*,
    secp256k1::utils::{parse_bytes, vec_to_array},
};
use solana_secp256k1_program::eth_address_from_pubkey;

const NAME: &str = "eth_address_from_pubkey";
const DEFINITION: &str = flow_lib::node_definition!("secp256k1/eth_address_from_pubkey.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    /// 64-byte uncompressed secp256k1 public key (without 0x04 prefix)
    pub pubkey: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    /// 20-byte Ethereum address as byte array
    pub eth_address: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Parse pubkey from JSON to Vec<u8>
    let pubkey_bytes = parse_bytes(&input.pubkey)?;
    let pubkey: [u8; 64] = vec_to_array(pubkey_bytes, "pubkey")?;

    // Derive Ethereum address using the secp256k1 library
    let eth_address: [u8; 20] = eth_address_from_pubkey(&pubkey);

    // Return as JSON array
    Ok(Output {
        eth_address: serde_json::json!(eth_address.to_vec()),
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
    async fn test_eth_address_derivation() {
        // Example 64-byte public key (test data)
        let pubkey: Vec<u8> = (0..64).collect();
        let input = Input {
            pubkey: serde_json::json!(pubkey),
        };

        let ctx = CommandContext::default();
        let result = run(ctx, input).await;

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let output = result.unwrap();

        // Verify address length (should be 20 bytes)
        let addr_array = output.eth_address.as_array().unwrap();
        assert_eq!(addr_array.len(), 20, "Ethereum address should be 20 bytes");
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "pubkey" => serde_json::json!(vec![0u8; 64]),
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
