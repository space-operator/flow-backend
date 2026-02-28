//! Swig Wallet nodes for Space Operator
//!
//! On-chain instruction nodes + Paymaster/Portal REST API nodes.


// Swig Wallet - Space Operator nodes
//
// Program ID: `swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB`
// Repository: https://github.com/anagrambuild/swig-wallet
// Documentation: https://build.onswig.com

use crate::prelude::*;

// =============================================================================
// Re-exports from swig-interface (instruction builders)
// =============================================================================

pub use swig_interface::{
    AuthorityConfig, ClientAction, CreateInstruction,
    AddAuthorityInstruction, RemoveAuthorityInstruction,
    UpdateAuthorityInstruction, UpdateAuthorityData,
    CreateSessionInstruction, CreateSubAccountInstruction,
    WithdrawFromSubAccountInstruction, SubAccountSignInstruction,
    ToggleSubAccountInstruction, TransferAssetsV1Instruction,
    CloseTokenAccountV1Instruction, CloseSwigV1Instruction,
    SignV2Instruction,
};

pub use swig_state::authority::AuthorityType;
use swig_state::action::{
    all::All, manage_authority::ManageAuthority,
    all_but_manage_authority::AllButManageAuthority,
    close_swig_authority::CloseSwigAuthority,
    program_all::ProgramAll, stake_all::StakeAll,
    sol_limit::SolLimit, token_limit::TokenLimit,
    program::Program, stake_limit::StakeLimit,
};

// =============================================================================
// Program Constants
// =============================================================================

/// Swig Wallet Program ID
pub const SWIG_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");

/// System Program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

// =============================================================================
// Solana v2 ↔ v3 Type Conversion
// =============================================================================

/// Convert solana-pubkey v3 Pubkey to solana-program v2 Pubkey.
/// Required because swig-interface uses solana-sdk v2 types.
#[inline]
pub fn to_pubkey_v2(pk: &solana_pubkey::Pubkey) -> solana_program_v2::pubkey::Pubkey {
    solana_program_v2::pubkey::Pubkey::new_from_array(pk.to_bytes())
}

/// Convert solana-program v2 Instruction to solana-instruction v3.
/// Required because swig-interface returns v2 Instructions.
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
// ClientAction Builder Helper
// =============================================================================

/// Build a Vec<ClientAction> from a permission type string and optional parameters.
/// Maps user-facing permission names to swig-interface `ClientAction` variants.
pub fn build_client_action(
    permission_type: &str,
    sol_limit: Option<u64>,
    token_mint: Option<&Pubkey>,
    token_limit: Option<u64>,
    program_id: Option<&Pubkey>,
) -> Vec<ClientAction> {
    let action = match permission_type {
        "all" => ClientAction::All(All),
        "manage_authority" => ClientAction::ManageAuthority(ManageAuthority),
        "all_but_manage_authority" => ClientAction::AllButManageAuthority(AllButManageAuthority),
        "close_swig_authority" => ClientAction::CloseSwigAuthority(CloseSwigAuthority),
        "program_all" => ClientAction::ProgramAll(ProgramAll),
        "stake_all" => ClientAction::StakeAll(StakeAll),
        "sol_limit" => ClientAction::SolLimit(SolLimit {
            amount: sol_limit.unwrap_or(0),
        }),
        "token_limit" => {
            let mint_bytes = token_mint
                .map(|pk| pk.to_bytes())
                .unwrap_or([0u8; 32]);
            ClientAction::TokenLimit(TokenLimit {
                token_mint: mint_bytes,
                current_amount: token_limit.unwrap_or(0),
            })
        }
        "program" => {
            let pid_bytes = program_id
                .map(|pk| pk.to_bytes())
                .unwrap_or([0u8; 32]);
            ClientAction::Program(Program {
                program_id: pid_bytes,
            })
        }
        "stake_limit" => ClientAction::StakeLimit(StakeLimit {
            amount: sol_limit.unwrap_or(0),
        }),
        // SubAccount has private _padding field; safely zero-initialize
        "sub_account" => ClientAction::SubAccount(unsafe { std::mem::zeroed() }),
        _ => ClientAction::All(All), // default fallback
    };
    vec![action]
}

// =============================================================================
// PDA Derivation Functions (v3 Pubkey)
// =============================================================================

/// Derive Swig account PDA from a 32-byte ID
pub fn find_swig_pda(id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"swig", id.as_ref()], &SWIG_PROGRAM_ID)
}

/// Derive Swig wallet address PDA from the Swig account pubkey
pub fn find_wallet_address(swig_account: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"swig-wallet-address", swig_account.as_ref()],
        &SWIG_PROGRAM_ID,
    )
}

/// Derive sub-account PDA from Swig ID and role ID
pub fn find_sub_account_pda(swig_id: &[u8; 32], role_id: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"sub-account", swig_id.as_ref(), &role_id.to_le_bytes()],
        &SWIG_PROGRAM_ID,
    )
}

// =============================================================================
// REST API Helpers
// =============================================================================

pub const PAYMASTER_BASE_URL: &str = "https://api.onswig.com";
pub const PORTAL_BASE_URL: &str = "https://dashboard.onswig.com/api/v1";

pub fn paymaster_post(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .post(format!("{PAYMASTER_BASE_URL}{path}"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
}

pub fn paymaster_get(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .get(format!("{PAYMASTER_BASE_URL}{path}"))
        .header("Authorization", format!("Bearer {api_key}"))
}

pub fn portal_post(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .post(format!("{PORTAL_BASE_URL}{path}"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
}

pub fn portal_get(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .get(format!("{PORTAL_BASE_URL}{path}"))
        .header("Authorization", format!("Bearer {api_key}"))
}

pub async fn check_response(resp: reqwest::Response) -> Result<JsonValue, CommandError> {
    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Swig API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    Ok(resp.json().await?)
}

// =============================================================================
// Swig Account Parsing
// =============================================================================

/// Parse Swig account data from raw bytes.
/// Layout: discriminator(1) + bump(1) + id(32) + roles(2) + role_counter(4) + wallet_bump(1) + padding(7) = 48 bytes header
pub fn parse_swig_account(data: &[u8]) -> Result<JsonValue, CommandError> {
    if data.len() < 48 {
        return Err(CommandError::msg("Swig account data too short (need >= 48 bytes)"));
    }
    let discriminator = data[0];
    if discriminator != 1 {
        return Err(CommandError::msg(format!(
            "Not a Swig account (discriminator={discriminator}, expected 1)"
        )));
    }
    let bump = data[1];
    let id = &data[2..34];
    let roles = u16::from_le_bytes([data[34], data[35]]);
    let role_counter = u32::from_le_bytes([data[36], data[37], data[38], data[39]]);
    let wallet_bump = data[40];

    Ok(serde_json::json!({
        "discriminator": discriminator,
        "bump": bump,
        "id": bs58::encode(id).into_string(),
        "roles": roles,
        "role_counter": role_counter,
        "wallet_bump": wallet_bump,
    }))
}

// =============================================================================
// Node Modules - On-chain Instructions
// =============================================================================

pub mod swig_create;
pub mod swig_add_authority;
pub mod swig_remove_authority;
pub mod swig_update_authority;
pub mod swig_create_session;
pub mod swig_create_sub_account;
pub mod swig_withdraw_from_sub_account;
pub mod swig_toggle_sub_account;
pub mod swig_sign;
pub mod swig_transfer_assets;
pub mod swig_close_token_account;
pub mod swig_close;
pub mod swig_migrate_wallet_address;

// =============================================================================
// Node Modules - RPC Read
// =============================================================================

pub mod swig_get_account;
pub mod swig_find_pda;
pub mod swig_find_wallet_address;

// =============================================================================
// Node Modules - REST API
// =============================================================================

pub mod swig_sponsor_transaction;
pub mod swig_sign_remote;
pub mod swig_paymaster_health;
pub mod swig_create_wallet_api;
pub mod swig_get_policy;
