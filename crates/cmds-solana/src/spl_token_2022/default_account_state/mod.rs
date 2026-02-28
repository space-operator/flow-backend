//! Default Account State extension nodes.

use serde::{Deserialize, Serialize};
use spl_token_2022_interface::state::AccountState;

/// Wrapper for `AccountState` that supports serde (the upstream type lacks it).
/// Only `Initialized` and `Frozen` are valid for default-account-state operations.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DefaultAccountState {
    Initialized,
    Frozen,
}

impl From<DefaultAccountState> for AccountState {
    fn from(value: DefaultAccountState) -> Self {
        match value {
            DefaultAccountState::Initialized => AccountState::Initialized,
            DefaultAccountState::Frozen => AccountState::Frozen,
        }
    }
}

pub mod initialize;
pub mod update;
