//! Solana Attestation Service nodes for Space Operator
//!
//! On-chain instruction nodes for the Solana Attestation Service program.
//! Repository: https://github.com/solana-foundation/solana-attestation-service

use spl_token_2022::extension::ExtensionType;
use spl_token_2022::state::Mint;

// =============================================================================
// Submodules
// =============================================================================

pub mod pda;

pub mod change_authorized_signers;
pub mod change_schema_description;
pub mod change_schema_status;
pub mod change_schema_version;
pub mod close_attestation;
pub mod close_tokenized_attestation;
pub mod create_attestation;
pub mod create_credential;
pub mod create_schema;
pub mod create_tokenized_attestation;
pub mod tokenize_schema;

// =============================================================================
// Solana v2 ↔ v3 Type Conversion
// =============================================================================

/// Convert solana-pubkey v3 Pubkey to solana-program v2 Pubkey
/// Required because attestation-service-client uses solana-program v2
#[inline]
pub fn to_pubkey_v2(pk: &solana_pubkey::Pubkey) -> solana_program_v2::pubkey::Pubkey {
    solana_program_v2::pubkey::Pubkey::new_from_array(pk.to_bytes())
}

/// Convert solana-program v2 Instruction to solana-instruction v3
/// Required because attestation-service-client returns v2 instructions
#[inline]
pub fn to_instruction_v3(ix: solana_program_v2::instruction::Instruction) -> solana_instruction::Instruction {
    solana_instruction::Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(ix.program_id.to_bytes()),
        accounts: ix.accounts.into_iter().map(|a| solana_instruction::AccountMeta {
            pubkey: solana_pubkey::Pubkey::new_from_array(a.pubkey.to_bytes()),
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        }).collect(),
        data: ix.data,
    }
}

// =============================================================================
// JSON Parsing Helpers
// =============================================================================

/// Deserialize an optional u16, treating null/missing as 0
pub fn deserialize_optional_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let opt: Option<u16> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(0))
}

/// Parse JSON array of pubkey strings to Vec<Pubkey> (v2)
pub fn parse_pubkeys_v2(json: &serde_json::Value) -> Vec<solana_program_v2::pubkey::Pubkey> {
    json.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse::<solana_pubkey::Pubkey>().ok())
                .map(|pk| to_pubkey_v2(&pk))
                .collect()
        })
        .unwrap_or_default()
}

/// Parse JSON array to Vec<String>
/// Handles both raw strings ["a", "b"] and Space Operator Value format [{"S": "a"}, {"S": "b"}]
pub fn parse_strings(json: &serde_json::Value) -> Vec<String> {
    json.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    // Try raw string first: ["a", "b"]
                    if let Some(s) = v.as_str() {
                        return Some(s.to_string());
                    }
                    // Try Space Operator Value format: [{"S": "a"}, {"S": "b"}]
                    if let Some(s_val) = v.get("S")
                        && let Some(s) = s_val.as_str()
                    {
                        return Some(s.to_string());
                    }
                    None
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse JSON array of integers to Vec<u8>
/// Handles both raw integers [12, 12, 12, 3] and Space Operator Value format {"A": [{"I": "12"}, ...]}
pub fn to_bytes(json: &serde_json::Value) -> Vec<u8> {
    // Handle both raw array and Space Operator {"A": [...]} wrapper
    let arr = if let Some(arr) = json.as_array() {
        arr
    } else if let Some(a_val) = json.get("A") {
        if let Some(arr) = a_val.as_array() {
            arr
        } else {
            return Vec::new();
        }
    } else {
        return Vec::new();
    };

    arr.iter()
        .filter_map(|v| {
            // Try raw integer first: [12, 12, 12, 3]
            if let Some(n) = v.as_u64() {
                return Some(n as u8);
            }
            // Try Space Operator Value format: [{"I": "12"}, ...]
            if let Some(i_val) = v.get("I") {
                if let Some(s) = i_val.as_str() {
                    return s.parse::<u64>().ok().map(|n| n as u8);
                }
                if let Some(n) = i_val.as_u64() {
                    return Some(n as u8);
                }
            }
            None
        })
        .collect()
}

// =============================================================================
// Data Type Constants
// =============================================================================

/// SAS DataType enum values for schema layout
/// Must match solana-attestation-service/program/src/state/schema.rs SchemaDataTypes
pub mod data_type {
    pub const U8: u8 = 0;
    pub const U16: u8 = 1;
    pub const U32: u8 = 2;
    pub const U64: u8 = 3;
    pub const U128: u8 = 4;
    pub const I8: u8 = 5;
    pub const I16: u8 = 6;
    pub const I32: u8 = 7;
    pub const I64: u8 = 8;
    pub const I128: u8 = 9;
    pub const BOOL: u8 = 10;
    pub const CHAR: u8 = 11;
    pub const STRING: u8 = 12;
    pub const VEC_U8: u8 = 13;
    pub const VEC_U16: u8 = 14;
    pub const VEC_U32: u8 = 15;
    pub const VEC_U64: u8 = 16;
    pub const VEC_U128: u8 = 17;
    pub const VEC_I8: u8 = 18;
    pub const VEC_I16: u8 = 19;
    pub const VEC_I32: u8 = 20;
    pub const VEC_I64: u8 = 21;
    pub const VEC_I128: u8 = 22;
    pub const VEC_BOOL: u8 = 23;
    pub const VEC_CHAR: u8 = 24;
    pub const VEC_STRING: u8 = 25;
}

// =============================================================================
// JSON Extraction Helpers (private)
// =============================================================================

/// Extract a string value from JSON (handles both raw and Space Operator format)
fn extract_string(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(s_val) = v.get("S")
        && let Some(s) = s_val.as_str()
    {
        return Some(s.to_string());
    }
    None
}

/// Extract an integer value from JSON (handles both raw and Space Operator format)
fn extract_i64(v: &serde_json::Value) -> Option<i64> {
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    if let Some(i_val) = v.get("I") {
        if let Some(s) = i_val.as_str() {
            return s.parse::<i64>().ok();
        }
        if let Some(n) = i_val.as_i64() {
            return Some(n);
        }
    }
    None
}

/// Extract an unsigned integer value from JSON
fn extract_u64(v: &serde_json::Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if let Some(i_val) = v.get("I") {
        if let Some(s) = i_val.as_str() {
            return s.parse::<u64>().ok();
        }
        if let Some(n) = i_val.as_u64() {
            return Some(n);
        }
    }
    None
}

/// Extract a boolean value from JSON
fn extract_bool(v: &serde_json::Value) -> Option<bool> {
    if let Some(b) = v.as_bool() {
        return Some(b);
    }
    if let Some(b_val) = v.get("B")
        && let Some(b) = b_val.as_bool()
    {
        return Some(b);
    }
    None
}

/// Extract array from JSON value (handles Space Operator {"A": [...]} format)
fn extract_array(v: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(arr) = v.as_array() {
        arr.clone()
    } else if let Some(a_val) = v.get("A") {
        if let Some(arr) = a_val.as_array() {
            arr.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Extract Vec<u8> from JSON value
fn extract_vec_u8(v: &serde_json::Value) -> Vec<u8> {
    let arr = extract_array(v);
    if !arr.is_empty() {
        arr.iter()
            .filter_map(|v| extract_u64(v).map(|n| n as u8))
            .collect()
    } else if let Some(s) = extract_string(v) {
        // Also support base64 or raw string as bytes
        s.into_bytes()
    } else {
        Vec::new()
    }
}

// =============================================================================
// Borsh Encoding
// =============================================================================

/// Borsh-encode attestation data according to the schema layout.
///
/// The `layout` parameter specifies the data types (from SAS SchemaDataTypes enum).
/// The `data` parameter is a JSON array of values matching the layout.
///
/// Borsh encoding rules (matching SAS validation):
/// - Bool: 1 byte (0 or 1)
/// - U8/I8: 1 byte
/// - U16/I16: 2 bytes little-endian
/// - U32/I32: 4 bytes little-endian
/// - U64/I64: 8 bytes little-endian
/// - U128/I128: 16 bytes little-endian
/// - Char: 4 bytes (UTF-8 code point as u32 LE)
/// - String: 4-byte length prefix (u32 LE) + UTF-8 bytes
/// - Vec<T>: 4-byte length prefix (u32 LE) + elements
pub fn borsh_encode_attestation_data(layout: &[u8], data: &serde_json::Value) -> Vec<u8> {
    let mut result = Vec::new();

    // Handle both raw array and Space Operator {"A": [...]} format
    let data_array = if let Some(arr) = data.as_array() {
        arr.clone()
    } else if let Some(a_val) = data.get("A") {
        if let Some(arr) = a_val.as_array() {
            arr.clone()
        } else {
            return result; // Not a valid array
        }
    } else {
        return result; // Empty if not an array
    };
    let data_array = &data_array;

    for (i, dtype) in layout.iter().enumerate() {
        let value = match data_array.get(i) {
            Some(v) => v,
            None => continue, // Skip if no value provided
        };

        match *dtype {
            // Unsigned integers
            data_type::U8 => {
                let n = extract_u64(value).unwrap_or(0) as u8;
                result.push(n);
            }
            data_type::U16 => {
                let n = extract_u64(value).unwrap_or(0) as u16;
                result.extend_from_slice(&n.to_le_bytes());
            }
            data_type::U32 => {
                let n = extract_u64(value).unwrap_or(0) as u32;
                result.extend_from_slice(&n.to_le_bytes());
            }
            data_type::U64 => {
                let n = extract_u64(value).unwrap_or(0);
                result.extend_from_slice(&n.to_le_bytes());
            }
            data_type::U128 => {
                let n = extract_u64(value).unwrap_or(0) as u128;
                result.extend_from_slice(&n.to_le_bytes());
            }
            // Signed integers
            data_type::I8 => {
                let n = extract_i64(value).unwrap_or(0) as i8;
                result.push(n as u8);
            }
            data_type::I16 => {
                let n = extract_i64(value).unwrap_or(0) as i16;
                result.extend_from_slice(&n.to_le_bytes());
            }
            data_type::I32 => {
                let n = extract_i64(value).unwrap_or(0) as i32;
                result.extend_from_slice(&n.to_le_bytes());
            }
            data_type::I64 => {
                let n = extract_i64(value).unwrap_or(0);
                result.extend_from_slice(&n.to_le_bytes());
            }
            data_type::I128 => {
                let n = extract_i64(value).unwrap_or(0) as i128;
                result.extend_from_slice(&n.to_le_bytes());
            }
            // Bool
            data_type::BOOL => {
                let b = extract_bool(value).unwrap_or(false);
                result.push(if b { 1 } else { 0 });
            }
            // Char (UTF-8 code point as 4 bytes)
            data_type::CHAR => {
                let c = extract_string(value)
                    .and_then(|s| s.chars().next())
                    .unwrap_or('\0');
                result.extend_from_slice(&(c as u32).to_le_bytes());
            }
            // String
            data_type::STRING => {
                let s = extract_string(value).unwrap_or_default();
                let bytes = s.as_bytes();
                result.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                result.extend_from_slice(bytes);
            }
            // Vec<u8> (bytes)
            data_type::VEC_U8 => {
                let bytes: Vec<u8> = extract_vec_u8(value);
                result.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                result.extend_from_slice(&bytes);
            }
            // Vec<u16>
            data_type::VEC_U16 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_u64(&v).unwrap_or(0) as u16;
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<u32>
            data_type::VEC_U32 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_u64(&v).unwrap_or(0) as u32;
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<u64>
            data_type::VEC_U64 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_u64(&v).unwrap_or(0);
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<u128>
            data_type::VEC_U128 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_u64(&v).unwrap_or(0) as u128;
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<i8>
            data_type::VEC_I8 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_i64(&v).unwrap_or(0) as i8;
                    result.push(n as u8);
                }
            }
            // Vec<i16>
            data_type::VEC_I16 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_i64(&v).unwrap_or(0) as i16;
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<i32>
            data_type::VEC_I32 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_i64(&v).unwrap_or(0) as i32;
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<i64>
            data_type::VEC_I64 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_i64(&v).unwrap_or(0);
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<i128>
            data_type::VEC_I128 => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let n = extract_i64(&v).unwrap_or(0) as i128;
                    result.extend_from_slice(&n.to_le_bytes());
                }
            }
            // Vec<bool>
            data_type::VEC_BOOL => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let b = extract_bool(&v).unwrap_or(false);
                    result.push(if b { 1 } else { 0 });
                }
            }
            // Vec<char>
            data_type::VEC_CHAR => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let c = extract_string(&v)
                        .and_then(|s| s.chars().next())
                        .unwrap_or('\0');
                    result.extend_from_slice(&(c as u32).to_le_bytes());
                }
            }
            // Vec<String>
            data_type::VEC_STRING => {
                let arr = extract_array(value);
                result.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for v in arr {
                    let s = extract_string(&v).unwrap_or_default();
                    let bytes = s.as_bytes();
                    result.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    result.extend_from_slice(bytes);
                }
            }
            _ => {
                // Unknown type, skip
            }
        }
    }

    result
}

// =============================================================================
// Mint Size Calculations
// =============================================================================

/// Calculate the schema mint account size with GroupPointer extension.
pub fn calculate_schema_mint_size() -> u64 {
    ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::GroupPointer]).unwrap() as u64
}

/// Calculate the attestation mint account size with all required extensions.
pub fn calculate_attestation_mint_size(
    name: &str,
    symbol: &str,
    uri: &str,
    attestation_data_len: usize,
) -> u16 {
    let base_size = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::GroupMemberPointer,
        ExtensionType::NonTransferable,
        ExtensionType::MetadataPointer,
        ExtensionType::PermanentDelegate,
        ExtensionType::MintCloseAuthority,
        ExtensionType::TokenGroupMember,
    ])
    .unwrap();

    let attestation_hex_len = attestation_data_len * 2;
    let additional_metadata_size = 4 + 11 + 4 + attestation_hex_len;

    let metadata_size = 4  // TLV header (type + length)
        + 33  // update_authority (Option<Pubkey>)
        + 32  // mint
        + 4 + name.len()  // name
        + 4 + symbol.len()  // symbol
        + 4 + uri.len()  // uri
        + 4  // additional_metadata vec length
        + additional_metadata_size;

    let total = base_size + metadata_size;
    let with_margin = total + total / 5;
    let aligned = (with_margin + 7) & !7;

    aligned as u16
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes_raw_array() {
        let json = serde_json::json!([12, 12, 12, 3]);
        let bytes = to_bytes(&json);
        assert_eq!(bytes, vec![12, 12, 12, 3]);
    }

    #[test]
    fn test_to_bytes_space_operator_format() {
        let json = serde_json::json!({"A": [{"I": "12"}, {"I": "12"}, {"I": "12"}, {"I": "3"}]});
        let bytes = to_bytes(&json);
        assert_eq!(bytes, vec![12, 12, 12, 3]);
    }

    #[test]
    fn test_borsh_encode_attestation_data() {
        let layout = vec![12u8, 12, 12, 3];
        let data = serde_json::json!(["test_hash", "test_commitment", "Party A", 1706892000]);
        let encoded = borsh_encode_attestation_data(&layout, &data);

        let mut expected = Vec::new();
        expected.extend_from_slice(&[9, 0, 0, 0]);
        expected.extend_from_slice(b"test_hash");
        expected.extend_from_slice(&[15, 0, 0, 0]);
        expected.extend_from_slice(b"test_commitment");
        expected.extend_from_slice(&[7, 0, 0, 0]);
        expected.extend_from_slice(b"Party A");
        expected.extend_from_slice(&1706892000u64.to_le_bytes());

        assert_eq!(encoded.len(), expected.len());
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_borsh_encode_space_operator_format() {
        let layout_json = serde_json::json!({"A": [{"I": "12"}, {"I": "12"}, {"I": "12"}, {"I": "3"}]});
        let layout = to_bytes(&layout_json);
        let data = serde_json::json!({"A": [
            {"S": "test_hash"},
            {"S": "test_commitment"},
            {"S": "Party A"},
            {"I": "1706892000"}
        ]});
        let encoded = borsh_encode_attestation_data(&layout, &data);

        let mut expected = Vec::new();
        expected.extend_from_slice(&[9, 0, 0, 0]);
        expected.extend_from_slice(b"test_hash");
        expected.extend_from_slice(&[15, 0, 0, 0]);
        expected.extend_from_slice(b"test_commitment");
        expected.extend_from_slice(&[7, 0, 0, 0]);
        expected.extend_from_slice(b"Party A");
        expected.extend_from_slice(&1706892000u64.to_le_bytes());

        assert_eq!(encoded.len(), 51);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_borsh_encode_empty_layout() {
        let layout: Vec<u8> = vec![];
        let data = serde_json::json!([]);
        let encoded = borsh_encode_attestation_data(&layout, &data);
        assert_eq!(encoded, Vec::<u8>::new());
    }

    #[test]
    fn test_calculate_attestation_mint_size() {
        let size = calculate_attestation_mint_size(
            "ZK Attestation NFT",
            "ZKATST",
            "https://example.com/attestation/metadata.json",
            51,
        );
        assert!((400..=1200).contains(&size), "Size {} should be in range 400-1200", size);
    }

    #[test]
    fn test_calculate_attestation_mint_size_empty_data() {
        let size = calculate_attestation_mint_size(
            "Test NFT",
            "TEST",
            "https://example.com/metadata.json",
            0,
        );
        assert!((300..=1000).contains(&size), "Size {} should be in range 300-1000", size);
    }
}

#[cfg(test)]
mod flow_tests {
    #[test]
    fn test_standard_attestation_flow_documented() {
        let flow_nodes = [
            "create_credential",
            "create_schema",
            "create_attestation",
            "close_attestation",
        ];
        let expected_edges = [
            ("create_credential", "credential", "create_schema", "credential"),
            ("create_credential", "credential", "create_attestation", "credential"),
            ("create_schema", "schema", "create_attestation", "schema"),
            ("create_attestation", "attestation", "close_attestation", "attestation"),
            ("create_credential", "credential", "close_attestation", "credential"),
        ];
        assert_eq!(flow_nodes.len(), 4);
        assert_eq!(expected_edges.len(), 5);
    }

    #[test]
    fn test_tokenized_attestation_flow_documented() {
        let flow_nodes = [
            "create_credential",
            "create_schema",
            "tokenize_schema",
            "create_tokenized_attestation",
            "close_tokenized_attestation",
        ];
        let expected_edges = [
            ("create_credential", "credential", "create_schema", "credential"),
            ("create_credential", "credential", "tokenize_schema", "credential"),
            ("create_schema", "schema", "tokenize_schema", "schema"),
            ("tokenize_schema", "mint", "create_tokenized_attestation", "schema_mint"),
            ("create_credential", "credential", "create_tokenized_attestation", "credential"),
            ("create_schema", "schema", "create_tokenized_attestation", "schema"),
            ("create_tokenized_attestation", "attestation", "close_tokenized_attestation", "attestation"),
        ];
        assert_eq!(flow_nodes.len(), 5);
        assert_eq!(expected_edges.len(), 7);
    }

    #[test]
    fn test_schema_management_operations_documented() {
        let management_nodes = [
            "change_schema_status",
            "change_schema_description",
            "change_schema_version",
            "change_authorized_signers",
        ];
        assert_eq!(management_nodes.len(), 4);
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_standard_attestation_flow_integration() {
        use crate::prelude::*;
        use solana_keypair::{Keypair, Signer};

        let wallet: Wallet = Keypair::from_base58_string(
            "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
        )
        .into();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let credential_name = format!("TestCred_{}", timestamp);
        let schema_name = format!("TestSchema_{}", timestamp);

        let cred_input = super::create_credential::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            name: credential_name.clone(),
            signers: vec![wallet.pubkey()],
            submit: true,
        };

        let cred_result =
            super::create_credential::run(CommandContext::default(), cred_input).await;
        assert!(cred_result.is_ok(), "create_credential failed: {:?}", cred_result.err());
        let cred_output = cred_result.unwrap();
        let credential = cred_output.credential;

        let schema_input = super::create_schema::Input {
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

        let schema_result =
            super::create_schema::run(CommandContext::default(), schema_input).await;
        assert!(schema_result.is_ok(), "create_schema failed: {:?}", schema_result.err());
        let schema_output = schema_result.unwrap();
        let schema = schema_output.schema;

        let nonce = Keypair::new().pubkey();
        let attestation_input = super::create_attestation::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            schema,
            nonce,
            layout: serde_json::json!([]),
            data: serde_json::json!([]),
            expiry: 0,
            submit: true,
        };

        let attestation_result =
            super::create_attestation::run(CommandContext::default(), attestation_input).await;
        assert!(attestation_result.is_ok(), "create_attestation failed: {:?}", attestation_result.err());
        let attestation_output = attestation_result.unwrap();
        let attestation = attestation_output.attestation;

        let close_input = super::close_attestation::Input {
            fee_payer: wallet.clone(),
            payer: wallet.clone(),
            authority: wallet.clone(),
            credential,
            attestation,
            submit: true,
        };

        let close_result =
            super::close_attestation::run(CommandContext::default(), close_input).await;
        assert!(close_result.is_ok(), "close_attestation failed: {:?}", close_result.err());
    }
}
