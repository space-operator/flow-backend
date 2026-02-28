//! TukTuk PDA derivation functions
//!
//! These functions derive Program Derived Addresses (PDAs) for TukTuk accounts.

use solana_program::hash::hash;
use solana_program::pubkey::Pubkey;

use super::TUKTUK_PROGRAM_ID;

/// Seed for TukTuk config PDA
pub const TUKTUK_CONFIG_SEED: &[u8] = b"tuktuk_config";
/// Seed for task queue PDA
pub const TASK_QUEUE_SEED: &[u8] = b"task_queue";
/// Seed for task queue name mapping PDA
pub const TASK_QUEUE_NAME_MAPPING_SEED: &[u8] = b"task_queue_name_mapping";
/// Seed for task queue authority PDA
pub const TASK_QUEUE_AUTHORITY_SEED: &[u8] = b"task_queue_authority";
/// Seed for task PDA
pub const TASK_SEED: &[u8] = b"task";

/// Find the TukTuk config PDA
pub fn find_tuktuk_config() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TUKTUK_CONFIG_SEED], &TUKTUK_PROGRAM_ID)
}

/// Find a task queue PDA by queue ID
pub fn find_task_queue(tuktuk_config: &Pubkey, queue_id: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TASK_QUEUE_SEED,
            tuktuk_config.as_ref(),
            &queue_id.to_le_bytes(),
        ],
        &TUKTUK_PROGRAM_ID,
    )
}

/// Find a task queue name mapping PDA
pub fn find_task_queue_name_mapping(tuktuk_config: &Pubkey, name: &str) -> (Pubkey, u8) {
    let name_hash = hash_name(name);
    Pubkey::find_program_address(
        &[
            TASK_QUEUE_NAME_MAPPING_SEED,
            tuktuk_config.as_ref(),
            &name_hash,
        ],
        &TUKTUK_PROGRAM_ID,
    )
}

/// Find a task queue authority PDA
pub fn find_task_queue_authority(task_queue: &Pubkey, queue_authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TASK_QUEUE_AUTHORITY_SEED,
            task_queue.as_ref(),
            queue_authority.as_ref(),
        ],
        &TUKTUK_PROGRAM_ID,
    )
}

/// Find a task PDA
pub fn find_task(task_queue: &Pubkey, task_id: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TASK_SEED, task_queue.as_ref(), &task_id.to_le_bytes()],
        &TUKTUK_PROGRAM_ID,
    )
}

/// Hash a name for PDA derivation using SHA-256
pub fn hash_name(name: &str) -> [u8; 32] {
    hash(name.as_bytes()).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_tuktuk_config() {
        let (pda, bump) = find_tuktuk_config();
        assert_ne!(pda, Pubkey::default());
        assert!(bump <= 255);
    }

    #[test]
    fn test_find_task_queue_authority() {
        let task_queue = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let (pda, bump) = find_task_queue_authority(&task_queue, &authority);
        assert_ne!(pda, Pubkey::default());
        assert!(bump <= 255);
    }

    #[test]
    fn test_hash_name_deterministic() {
        let hash1 = hash_name("my_queue");
        let hash2 = hash_name("my_queue");
        assert_eq!(hash1, hash2);
    }
}
