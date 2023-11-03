use crate::prelude::Pubkey;
use serde::{Deserialize, Serialize};

use clockwork_client::thread::state::ThreadSettings as ClockWorkThreadSettings;
use clockwork_utils::thread::SerializableAccount as ClockWorkAccount;
use clockwork_utils::thread::SerializableInstruction as ClockWorkInstruction;
use clockwork_utils::thread::Trigger as ClockWorkTrigger;

pub mod thread_create;
pub mod thread_delete;
pub mod thread_pause;
pub mod thread_reset;
pub mod thread_resume;
pub mod thread_update;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<Instruction> for ClockWorkInstruction {
    fn from(instruction: Instruction) -> Self {
        ClockWorkInstruction {
            program_id: instruction.program_id,
            accounts: instruction
                .accounts
                .iter()
                .map(|a| ClockWorkAccount {
                    pubkey: a.pubkey,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect(),
            data: instruction.data,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum Trigger {
    /// Allows a thread to be kicked off whenever the data of an account changes.
    Account {
        /// The address of the account to monitor.
        address: Pubkey,
        /// The byte offset of the account data to monitor.
        offset: u64,
        /// The size of the byte slice to monitor (must be less than 1kb)
        size: u64,
    },

    /// Allows an thread to be kicked off according to a one-time or recurring schedule.
    Cron {
        /// The schedule in cron syntax. Value must be parsable by the `clockwork_cron` package.
        schedule: String,

        /// Boolean value indicating whether triggering moments may be skipped if they are missed (e.g. due to network downtime).
        /// If false, any "missed" triggering moments will simply be executed as soon as the network comes back online.
        skippable: bool,
    },

    /// Allows an thread to be kicked off as soon as it's created.
    Now,

    /// Allows a thread to be kicked off according to a slot.
    Slot { slot: u64 },

    /// Allows a thread to be kicked off according to an epoch number.
    Epoch { epoch: u64 },
}

// Implement From  Trigger to ClockWorkTrigger
impl From<Trigger> for ClockWorkTrigger {
    fn from(trigger: Trigger) -> Self {
        match trigger {
            Trigger::Account {
                address,
                offset,
                size,
            } => ClockWorkTrigger::Account {
                address,
                offset,
                size,
            },
            Trigger::Cron {
                schedule,
                skippable,
            } => ClockWorkTrigger::Cron {
                schedule,
                skippable,
            },
            Trigger::Now => ClockWorkTrigger::Now,
            Trigger::Slot { slot } => ClockWorkTrigger::Slot { slot },
            Trigger::Epoch { epoch } => ClockWorkTrigger::Epoch { epoch },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThreadSettings {
    pub fee: Option<u64>,
    pub instructions: Option<Vec<Instruction>>,
    pub name: Option<String>,
    pub rate_limit: Option<u64>,
    pub trigger: Option<Trigger>,
}

// Implement From ThreadSettings to ClockWorkThreadSettings
impl From<ThreadSettings> for ClockWorkThreadSettings {
    fn from(thread_settings: ThreadSettings) -> Self {
        ClockWorkThreadSettings {
            fee: thread_settings.fee,
            instructions: thread_settings
                .instructions
                .map(|i| i.into_iter().map(|i| i.into()).collect()),
            name: thread_settings.name,
            rate_limit: thread_settings.rate_limit,
            trigger: thread_settings.trigger.map(|t| t.into()),
        }
    }
}
