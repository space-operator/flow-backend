//! Utility functions for secp256k1 node implementations

use anyhow::{anyhow, Result};

/// Convert Vec<u8> to fixed-size array with descriptive error message
pub fn vec_to_array<const N: usize>(v: Vec<u8>, field_name: &str) -> Result<[u8; N]> {
    v.try_into()
        .map_err(|v: Vec<u8>| anyhow!(
            "{} must be exactly {} bytes, got {} bytes",
            field_name, N, v.len()
        ))
}

/// Parse bytes from JSON value
///
/// Supports:
/// - JSON arrays: `[1, 2, 3, ...]`
/// - Hex strings: `"0x010203..."` or `"010203..."`
pub fn parse_bytes(value: &serde_json::Value) -> Result<Vec<u8>> {
    match value {
        serde_json::Value::Array(arr) => {
            arr.iter()
                .map(|v| {
                    v.as_u64()
                        .ok_or_else(|| anyhow!("invalid byte value: expected number"))
                        .and_then(|n| {
                            u8::try_from(n).map_err(|_| anyhow!("byte value {} out of range 0-255", n))
                        })
                })
                .collect()
        }
        serde_json::Value::String(s) => {
            let s = s.strip_prefix("0x").unwrap_or(s);
            hex::decode(s).map_err(|e| anyhow!("invalid hex string: {}", e))
        }
        _ => Err(anyhow!("expected byte array or hex string, got {:?}", value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_to_array_success() {
        let v = vec![1u8, 2, 3, 4];
        let arr: [u8; 4] = vec_to_array(v, "test").unwrap();
        assert_eq!(arr, [1, 2, 3, 4]);
    }

    #[test]
    fn test_vec_to_array_wrong_size() {
        let v = vec![1u8, 2, 3];
        let result: Result<[u8; 4]> = vec_to_array(v, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be exactly 4 bytes"));
    }

    #[test]
    fn test_parse_bytes_array() {
        let json = serde_json::json!([1, 2, 3, 255]);
        let result = parse_bytes(&json).unwrap();
        assert_eq!(result, vec![1, 2, 3, 255]);
    }

    #[test]
    fn test_parse_bytes_hex_with_prefix() {
        let json = serde_json::json!("0x010203ff");
        let result = parse_bytes(&json).unwrap();
        assert_eq!(result, vec![1, 2, 3, 255]);
    }

    #[test]
    fn test_parse_bytes_hex_without_prefix() {
        let json = serde_json::json!("010203ff");
        let result = parse_bytes(&json).unwrap();
        assert_eq!(result, vec![1, 2, 3, 255]);
    }

    #[test]
    fn test_parse_bytes_invalid_type() {
        let json = serde_json::json!(123);
        let result = parse_bytes(&json);
        assert!(result.is_err());
    }
}
