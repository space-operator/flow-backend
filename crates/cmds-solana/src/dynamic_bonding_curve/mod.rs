//! Auto-generated Space Operator nodes


use solana_program::pubkey::Pubkey;
use solana_pubkey::pubkey;

// Program constants
pub const DBC_PROGRAM_ID: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
pub const POOL_AUTHORITY: Pubkey = pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");

// PDA derivation functions
pub mod pda {
    use super::{DBC_PROGRAM_ID, Pubkey};

    /// Event authority PDA
    /// Seeds: "__event_authority"
    pub fn event_authority() -> Pubkey {
        Pubkey::find_program_address(&[b"__event_authority"], &DBC_PROGRAM_ID).0
    }

    /// Pool PDA
    /// Seeds: "pool" + config + base_mint + quote_mint
    pub fn pool(config: &Pubkey, base_mint: &Pubkey, quote_mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"pool", config.as_ref(), base_mint.as_ref(), quote_mint.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Base vault PDA
    /// Seeds: "token_vault" + base_mint + pool (IDL order)
    pub fn base_vault(base_mint: &Pubkey, pool: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"token_vault", base_mint.as_ref(), pool.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Quote vault PDA
    /// Seeds: "token_vault" + quote_mint + pool (IDL order)
    pub fn quote_vault(quote_mint: &Pubkey, pool: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"token_vault", quote_mint.as_ref(), pool.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Virtual pool metadata PDA
    /// Seeds: "virtual_pool_metadata" + virtual_pool
    pub fn virtual_pool_metadata(virtual_pool: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"virtual_pool_metadata", virtual_pool.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Partner metadata PDA
    /// Seeds: "partner_metadata" + fee_claimer
    pub fn partner_metadata(fee_claimer: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"partner_metadata", fee_claimer.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Claim fee operator PDA
    /// Seeds: "cf_operator" + operator (IDL name: create_claim_protocol_fee_operator)
    pub fn claim_fee_operator(operator: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"cf_operator", operator.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Base locker PDA (used by create_locker)
    /// Seeds: "base_locker" + virtual_pool
    pub fn base_locker(virtual_pool: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"base_locker", virtual_pool.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }

    /// Migration metadata PDA
    /// Seeds: "meteora" + virtual_pool
    pub fn migration_metadata(virtual_pool: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"meteora", virtual_pool.as_ref()],
            &DBC_PROGRAM_ID,
        ).0
    }
}

// Instruction discriminators (from IDL)
pub mod discriminators {
    pub const CLAIM_CREATOR_TRADING_FEE: [u8; 8] = [82, 220, 250, 189, 3, 85, 107, 45];
    pub const CLAIM_PARTNER_POOL_CREATION_FEE: [u8; 8] = [250, 238, 26, 4, 139, 10, 101, 248];
    pub const CLAIM_PROTOCOL_FEE: [u8; 8] = [165, 228, 133, 48, 99, 249, 255, 33];
    pub const CLAIM_PROTOCOL_POOL_CREATION_FEE: [u8; 8] = [114, 205, 83, 188, 240, 153, 25, 54];
    pub const CLAIM_TRADING_FEE: [u8; 8] = [8, 236, 89, 49, 152, 125, 177, 81];
    pub const CLOSE_CLAIM_FEE_OPERATOR: [u8; 8] = [8, 41, 87, 35, 80, 48, 121, 26];
    pub const CREATE_CLAIM_FEE_OPERATOR: [u8; 8] = [51, 19, 150, 252, 105, 157, 48, 91];
    pub const CREATE_CONFIG: [u8; 8] = [201, 207, 243, 114, 75, 111, 47, 189];
    pub const CREATE_LOCKER: [u8; 8] = [167, 90, 137, 154, 75, 47, 17, 84];
    pub const CREATE_PARTNER_METADATA: [u8; 8] = [192, 168, 234, 191, 188, 226, 227, 255];
    pub const CREATE_VIRTUAL_POOL_METADATA: [u8; 8] = [45, 97, 187, 103, 254, 109, 124, 134];
    pub const CREATOR_WITHDRAW_SURPLUS: [u8; 8] = [165, 3, 137, 7, 28, 134, 76, 80];
    pub const INITIALIZE_VIRTUAL_POOL_WITH_SPL_TOKEN: [u8; 8] = [140, 85, 215, 176, 102, 54, 104, 79];
    pub const INITIALIZE_VIRTUAL_POOL_WITH_TOKEN2022: [u8; 8] = [169, 118, 51, 78, 145, 110, 220, 155];
    pub const MIGRATE_METEORA_DAMM: [u8; 8] = [27, 1, 48, 22, 180, 63, 118, 217];
    pub const MIGRATE_METEORA_DAMM_CLAIM_LP_TOKEN: [u8; 8] = [139, 133, 2, 30, 91, 145, 127, 154];
    pub const MIGRATE_METEORA_DAMM_LOCK_LP_TOKEN: [u8; 8] = [177, 55, 238, 157, 251, 88, 165, 42];
    pub const MIGRATION_DAMM_V2: [u8; 8] = [156, 169, 230, 103, 53, 228, 80, 64];
    pub const MIGRATION_DAMM_V2_CREATE_METADATA: [u8; 8] = [109, 189, 19, 36, 195, 183, 222, 82];
    pub const MIGRATION_METEORA_DAMM_CREATE_METADATA: [u8; 8] = [47, 94, 126, 115, 221, 226, 194, 133];
    pub const PARTNER_WITHDRAW_SURPLUS: [u8; 8] = [168, 173, 72, 100, 201, 98, 38, 92];
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const SWAP2: [u8; 8] = [65, 75, 63, 76, 235, 91, 91, 136];
    pub const TRANSFER_POOL_CREATOR: [u8; 8] = [20, 7, 169, 33, 58, 147, 166, 33];
    pub const WITHDRAW_LEFTOVER: [u8; 8] = [20, 198, 202, 237, 235, 243, 183, 66];
    pub const WITHDRAW_MIGRATION_FEE: [u8; 8] = [237, 142, 45, 23, 129, 6, 222, 162];
}

pub mod claim_creator_trading_fee;
pub mod claim_partner_pool_creation_fee;
pub mod claim_protocol_fee;
pub mod claim_protocol_pool_creation_fee;
pub mod claim_trading_fee;
pub mod close_claim_fee_operator;
pub mod create_claim_fee_operator;
pub mod create_config;
pub mod create_locker;
pub mod create_partner_metadata;
pub mod create_virtual_pool_metadata;
pub mod creator_withdraw_surplus;
pub mod initialize_virtual_pool_with_spl_token;
pub mod initialize_virtual_pool_with_token2022;
pub mod migrate_meteora_damm;
pub mod migrate_meteora_damm_claim_lp_token;
pub mod migrate_meteora_damm_lock_lp_token;
pub mod migration_damm_v2;
pub mod migration_damm_v2_create_metadata;
pub mod migration_meteora_damm_create_metadata;
pub mod partner_withdraw_surplus;
pub mod swap;
pub mod swap2;
pub mod transfer_pool_creator;
pub mod withdraw_leftover;
pub mod withdraw_migration_fee;
