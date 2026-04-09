use crate::prelude::*;
use spl_associated_token_account_interface::address::get_associated_token_address;

/// Derive the Associated Token Account address for an SPL Token mint.
pub fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address(wallet, mint)
}

pub mod approve_checked;
pub mod associated_token_account;
pub mod burn_checked;
pub mod close_account;
pub mod create_mint_account;
pub mod create_token_account;
pub mod freeze_account;
pub mod mint_token;
pub mod revoke;
pub mod set_authority;
pub mod sync_native;
pub mod thaw_account;
pub mod transfer_checked;
