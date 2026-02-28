//! TukTuk shared types for serde (de)serialization
//!
//! These are the node-local types used for Input/Output structs.
//! They are converted to the tuktuk-program v2 types when building instructions.

use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;

/// Trigger type for when a task should execute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TriggerV0 {
    /// Execute at a specific Unix timestamp
    Timestamp { unix_timestamp: i64 },
    /// Execute immediately when cranked
    #[default]
    Now,
}

/// Transaction source - where to get the instructions to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TransactionSourceV0 {
    /// Pre-compiled transaction bytes stored on-chain
    CompiledV0 {
        num_rw_signers: u8,
        num_ro_signers: u8,
        num_rw: u8,
        data: Vec<u8>,
    },
    /// Fetch transaction from a remote URL at execution time
    RemoteV0 {
        url: String,
        signer: Pubkey,
    },
}

/// Task return value for recursive tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReturnV0 {
    /// Trigger for the new task
    pub trigger: TriggerV0,
    /// Transaction for the new task
    pub transaction: TransactionSourceV0,
    /// Crank reward override
    pub crank_reward: Option<u64>,
    /// Number of free tasks to allocate
    pub free_tasks: u8,
    /// Description
    pub description: String,
}
