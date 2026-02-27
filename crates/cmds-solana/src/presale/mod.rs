use crate::prelude::*;
use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
use solana_program::pubkey;

// presale - Space Operator nodes for Meteora Presale
//
// Program ID: `presSVxnf9UU8jMxhgSMqaRwNiT36qeBdNeTRKjTdbj`
// Repository: https://github.com/MeteoraAg/presale

// =============================================================================
// Program Constants
// =============================================================================

/// Meteora Presale Program ID
pub const PRESALE_PROGRAM_ID: Pubkey = pubkey!("presSVxnf9UU8jMxhgSMqaRwNiT36qeBdNeTRKjTdbj");

// =============================================================================
// PDA Derivation Functions
// =============================================================================

/// Derive event authority PDA (Anchor CPI events)
pub fn derive_event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &PRESALE_PROGRAM_ID).0
}

/// Derive presale authority PDA (global, no per-presale seeds)
pub fn derive_presale_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"presale_authority"], &PRESALE_PROGRAM_ID).0
}

/// Derive presale PDA from base key, presale_mint, and quote_token_mint
pub fn derive_presale(base: &Pubkey, presale_mint: &Pubkey, quote_token_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"presale",
            base.as_ref(),
            presale_mint.as_ref(),
            quote_token_mint.as_ref(),
        ],
        &PRESALE_PROGRAM_ID,
    )
    .0
}

/// Derive base token vault PDA from presale
pub fn derive_base_vault(presale: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"base_vault", presale.as_ref()], &PRESALE_PROGRAM_ID).0
}

/// Derive quote token vault PDA from presale
pub fn derive_quote_vault(presale: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"quote_vault", presale.as_ref()], &PRESALE_PROGRAM_ID).0
}

/// Derive escrow PDA from presale, owner, and registry index
pub fn derive_escrow(presale: &Pubkey, owner: &Pubkey, registry_index: u8) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"escrow".as_ref(),
            presale.as_ref(),
            owner.as_ref(),
            &[registry_index],
        ],
        &PRESALE_PROGRAM_ID,
    )
    .0
}

/// Derive fixed price presale args PDA
pub fn derive_fixed_price_args(presale: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"fixed_price_param", presale.as_ref()],
        &PRESALE_PROGRAM_ID,
    )
    .0
}

/// Derive merkle root config PDA
pub fn derive_merkle_root_config(presale: &Pubkey, version: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[b"merkle_root", presale.as_ref(), &version.to_le_bytes()],
        &PRESALE_PROGRAM_ID,
    )
    .0
}

/// Derive operator PDA
pub fn derive_operator(creator: &Pubkey, operator: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"operator", creator.as_ref(), operator.as_ref()],
        &PRESALE_PROGRAM_ID,
    )
    .0
}

/// Derive permissioned server metadata PDA
pub fn derive_server_metadata(presale: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"server_metadata", presale.as_ref()], &PRESALE_PROGRAM_ID).0
}

// =============================================================================
// Instruction Discriminators (from IDL)
// =============================================================================

pub mod discriminators {
    pub const CLAIM: [u8; 8] = [62, 198, 214, 193, 213, 159, 108, 210];
    pub const CLOSE_ESCROW: [u8; 8] = [139, 171, 94, 146, 191, 91, 144, 50];
    pub const CLOSE_FIXED_PRICE_PRESALE_ARGS: [u8; 8] = [125, 65, 70, 247, 99, 200, 42, 225];
    pub const CLOSE_MERKLE_ROOT_CONFIG: [u8; 8] = [157, 174, 38, 193, 204, 253, 3, 12];
    pub const CLOSE_PERMISSIONED_SERVER_METADATA: [u8; 8] = [226, 2, 147, 220, 38, 247, 138, 95];
    pub const CREATE_MERKLE_ROOT_CONFIG: [u8; 8] = [55, 243, 253, 240, 78, 186, 232, 166];
    pub const CREATE_OPERATOR: [u8; 8] = [145, 40, 238, 75, 181, 252, 59, 11];
    pub const CREATE_PERMISSIONED_ESCROW_WITH_CREATOR: [u8; 8] =
        [131, 130, 26, 39, 200, 38, 18, 173];
    pub const CREATE_PERMISSIONED_ESCROW_WITH_MERKLE_PROOF: [u8; 8] =
        [62, 200, 54, 145, 59, 239, 91, 5];
    pub const CREATE_PERMISSIONED_SERVER_METADATA: [u8; 8] = [139, 13, 120, 145, 18, 209, 185, 180];
    pub const CREATE_PERMISSIONLESS_ESCROW: [u8; 8] = [241, 26, 9, 26, 248, 201, 151, 0];
    pub const CREATOR_COLLECT_FEE: [u8; 8] = [9, 215, 62, 151, 64, 163, 150, 7];
    pub const CREATOR_WITHDRAW: [u8; 8] = [92, 117, 206, 254, 174, 108, 37, 106];
    pub const DEPOSIT: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
    pub const INITIALIZE_FIXED_PRICE_PRESALE_ARGS: [u8; 8] =
        [224, 80, 127, 193, 204, 143, 243, 194];
    pub const INITIALIZE_PRESALE: [u8; 8] = [9, 174, 12, 126, 150, 119, 68, 100];
    pub const PERFORM_UNSOLD_BASE_TOKEN_ACTION: [u8; 8] = [101, 141, 8, 65, 176, 225, 47, 110];
    pub const REFRESH_ESCROW: [u8; 8] = [68, 237, 17, 237, 147, 201, 27, 169];
    pub const REVOKE_OPERATOR: [u8; 8] = [185, 25, 87, 77, 88, 8, 30, 175];
    pub const WITHDRAW: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
    pub const WITHDRAW_REMAINING_QUOTE: [u8; 8] = [54, 253, 188, 34, 100, 145, 59, 127];
}

// =============================================================================
// Instruction Argument Types (borsh-serialized for instruction data)
// =============================================================================

/// Transfer hook account type for remaining accounts
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub enum AccountsType {
    TransferHookBase,
    TransferHookQuote,
}

/// A slice of remaining accounts for transfer hooks
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct RemainingAccountsSlice {
    pub accounts_type: AccountsType,
    pub length: u8,
}

/// Wrapper for remaining accounts info (borsh-serialized in instruction data)
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct RemainingAccountsInfo {
    pub slices: Vec<RemainingAccountsSlice>,
}

/// Presale parameters for initialize_presale
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct PresaleArgs {
    pub presale_maximum_cap: u64,
    pub presale_minimum_cap: u64,
    pub presale_start_time: u64,
    pub presale_end_time: u64,
    pub whitelist_mode: u8,
    pub presale_mode: u8,
    pub unsold_token_action: u8,
    pub disable_earlier_presale_end_once_cap_reached: u8,
    pub padding: [u8; 30],
}

/// Locked vesting parameters for initialize_presale
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct LockedVestingArgs {
    pub immediately_release_bps: u16,
    pub lock_duration: u64,
    pub vest_duration: u64,
    pub immediate_release_timestamp: u64,
    pub padding: [u8; 24],
}

/// Per-registry args for initialize_presale
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct PresaleRegistryArgs {
    pub buyer_minimum_deposit_cap: u64,
    pub buyer_maximum_deposit_cap: u64,
    pub presale_supply: u64,
    pub deposit_fee_bps: u16,
    pub padding: [u8; 32],
}

/// Full args for initialize_presale instruction
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct InitializePresaleArgs {
    pub presale_params: PresaleArgs,
    pub locked_vesting_params: LockedVestingArgs,
    pub padding: [u8; 32],
    pub presale_registries: Vec<PresaleRegistryArgs>,
}

/// Params for create_merkle_root_config
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct CreateMerkleRootConfigParams {
    pub root: [u8; 32],
    pub version: u64,
}

/// Params for create_permissioned_escrow_with_creator
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct CreatePermissionedEscrowWithCreatorParams {
    pub registry_index: u8,
    pub deposit_cap: u64,
    pub padding: [u8; 32],
}

/// Params for create_permissioned_escrow_with_merkle_proof
#[derive(BorshSerialize, Serialize, Deserialize, Debug, Clone)]
pub struct CreatePermissionedEscrowWithMerkleProofParams {
    pub proof: Vec<[u8; 32]>,
    pub registry_index: u8,
    pub deposit_cap: u64,
    pub padding: [u8; 32],
}

/// Params for initialize_fixed_price_presale_args
/// Note: Cannot derive BorshSerialize due to Pubkey borsh version mismatch.
/// Manually serialized in the instruction node.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitializeFixedPricePresaleExtraArgs {
    pub presale: Pubkey,
    pub disable_withdraw: u8,
    pub q_price: u128,
    pub padding1: [u64; 8],
}

// =============================================================================
// Presale Account Fetching (zero-copy / bytemuck)
// =============================================================================

/// Minimal presale account data for fetching mints.
/// Uses raw byte reading since the on-chain struct is zero-copy (bytemuck).
///
/// Layout (from program source, 8-byte Anchor discriminator then):
/// - offset 0: owner (Pubkey, 32 bytes)
/// - offset 32: quote_mint (Pubkey, 32 bytes)
/// - offset 64: base_mint (Pubkey, 32 bytes)
#[derive(Debug)]
pub struct PresaleAccountData {
    pub owner: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint: Pubkey,
}

impl PresaleAccountData {
    /// Deserialize presale account data from raw bytes.
    /// Reads directly from the zero-copy layout (no borsh).
    pub fn from_account_data(data: &[u8]) -> Result<Self, CommandError> {
        if data.len() < 8 + 32 + 32 + 32 {
            return Err(CommandError::msg("Presale account data too short"));
        }

        // Skip 8-byte Anchor discriminator
        let data = &data[8..];

        let owner = Pubkey::try_from(&data[0..32])
            .map_err(|_| CommandError::msg("Invalid owner pubkey"))?;
        let quote_mint = Pubkey::try_from(&data[32..64])
            .map_err(|_| CommandError::msg("Invalid quote_mint pubkey"))?;
        let base_mint = Pubkey::try_from(&data[64..96])
            .map_err(|_| CommandError::msg("Invalid base_mint pubkey"))?;

        Ok(Self {
            owner,
            quote_mint,
            base_mint,
        })
    }
}

/// Fetch presale account data to get base_mint and quote_mint
pub async fn fetch_presale_account(
    ctx: &CommandContext,
    presale: &Pubkey,
) -> Result<PresaleAccountData, CommandError> {
    let rpc = ctx.solana_client();

    let account = rpc
        .get_account(presale)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to fetch presale account: {}", e)))?;

    PresaleAccountData::from_account_data(&account.data)
}

// =============================================================================
// Node Modules
// =============================================================================

pub mod claim;
pub mod close_escrow;
pub mod close_fixed_price_presale_args;
pub mod close_merkle_root_config;
pub mod close_permissioned_server_metadata;
pub mod create_merkle_root_config;
pub mod create_operator;
pub mod create_permissioned_escrow_with_creator;
pub mod create_permissioned_escrow_with_merkle_proof;
pub mod create_permissioned_server_metadata;
pub mod create_permissionless_escrow;
pub mod creator_collect_fee;
pub mod creator_withdraw;
pub mod deposit;
pub mod get_escrow;
pub mod get_presale;
pub mod initialize_fixed_price_presale_args;
pub mod initialize_presale;
pub mod perform_unsold_base_token_action;
pub mod refresh_escrow;
pub mod revoke_operator;
pub mod withdraw;
pub mod withdraw_remaining_quote;
