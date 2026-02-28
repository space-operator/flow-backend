//! Jupiter Locker (jup-lock) Space Operator nodes
//!
//! Program ID: `LocpQgucEQHbqNABEYvBvwoxCPsSbG91A1QaQhQQqjn`
//! Repository: https://github.com/jup-ag/jup-lock
//!
//! Direct instruction construction (no SDK crate dependency).
//! Computes Anchor discriminators via SHA-256 and borsh-serializes args manually.

use crate::prelude::*;
use solana_program::pubkey;

pub mod claim;
pub mod claim_v2;
pub mod create_vesting_escrow;
pub mod create_vesting_escrow_metadata;
pub mod create_vesting_escrow_v2;
pub mod pda;
pub mod update_vesting_escrow_recipient;

/// Jupiter Locker program ID (mainnet production)
pub const JUP_LOCK_PROGRAM_ID: Pubkey = pubkey!("LocpQgucEQHbqNABEYvBvwoxCPsSbG91A1QaQhQQqjn");

/// Borsh-encode a String: 4-byte LE length prefix + UTF-8 bytes
pub fn borsh_string(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(4 + bytes.len());
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    out
}

/// Borsh-encode an Option<String>: 0u8 for None, 1u8 + borsh(String) for Some
pub fn borsh_option_string(opt: &Option<String>) -> Vec<u8> {
    match opt {
        None => vec![0u8],
        Some(s) => {
            let mut out = vec![1u8];
            out.extend(borsh_string(s));
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borsh_string() {
        let encoded = borsh_string("hello");
        assert_eq!(&encoded[..4], &[5, 0, 0, 0]); // length = 5
        assert_eq!(&encoded[4..], b"hello");
    }

    #[test]
    fn test_borsh_string_empty() {
        let encoded = borsh_string("");
        assert_eq!(encoded, vec![0, 0, 0, 0]); // length = 0, no bytes
    }

    #[test]
    fn test_borsh_option_string_none() {
        assert_eq!(borsh_option_string(&None), vec![0u8]);
    }

    #[test]
    fn test_borsh_option_string_some() {
        let encoded = borsh_option_string(&Some("hi".to_string()));
        assert_eq!(&encoded[..1], &[1u8]); // Some tag
        assert_eq!(&encoded[1..5], &[2, 0, 0, 0]); // length = 2
        assert_eq!(&encoded[5..], b"hi");
    }

    #[test]
    fn test_borsh_option_string_some_empty() {
        let encoded = borsh_option_string(&Some(String::new()));
        // Some tag + 4-byte zero length
        assert_eq!(encoded, vec![1, 0, 0, 0, 0]);
    }
}
