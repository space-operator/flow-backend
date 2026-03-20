//! Solana Escrow Program Space Operator nodes
//!
//! Program ID: `Escrowae7RaUfNn4oEZHywMXE5zWzYCXenwrCDaEoifg`
//! Repository: https://github.com/solana-program/escrow
//!
//! Direct instruction construction (no SDK crate dependency).
//! This is a Pinocchio-based program using single-byte discriminators.

use crate::prelude::*;
use solana_program::pubkey;

pub mod add_timelock;
pub mod allow_mint;
pub mod block_mint;
pub mod block_token_extension;
pub mod create_escrow;
pub mod deposit;
pub mod pda;
pub mod set_arbiter;
pub mod set_hook;
pub mod update_admin;
pub mod withdraw;

/// Escrow program ID (mainnet / devnet)
pub const ESCROW_PROGRAM_ID: Pubkey = pubkey!("Escrowae7RaUfNn4oEZHywMXE5zWzYCXenwrCDaEoifg");

/// Default token program (SPL Token)
pub const DEFAULT_TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Serde default: returns the SPL Token program ID
pub fn default_token_program() -> Pubkey {
    DEFAULT_TOKEN_PROGRAM
}

/// Escrow instruction discriminators (single byte, Pinocchio-style)
#[repr(u8)]
pub enum EscrowDiscriminator {
    CreateEscrow = 0,
    AddTimelock = 1,
    SetHook = 2,
    Deposit = 3,
    UpdateAdmin = 4,
    Withdraw = 5,
    AllowMint = 6,
    BlockMint = 7,
    BlockTokenExtension = 8,
    SetArbiter = 9,
}

/// Build an escrow instruction: 1-byte discriminator + args data.
pub fn build_escrow_instruction(
    discriminator: EscrowDiscriminator,
    accounts: Vec<solana_program::instruction::AccountMeta>,
    args_data: Vec<u8>,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + args_data.len());
    data.push(discriminator as u8);
    data.extend_from_slice(&args_data);
    Instruction {
        program_id: ESCROW_PROGRAM_ID,
        accounts,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_values() {
        assert_eq!(EscrowDiscriminator::CreateEscrow as u8, 0);
        assert_eq!(EscrowDiscriminator::AddTimelock as u8, 1);
        assert_eq!(EscrowDiscriminator::SetHook as u8, 2);
        assert_eq!(EscrowDiscriminator::Deposit as u8, 3);
        assert_eq!(EscrowDiscriminator::UpdateAdmin as u8, 4);
        assert_eq!(EscrowDiscriminator::Withdraw as u8, 5);
        assert_eq!(EscrowDiscriminator::AllowMint as u8, 6);
        assert_eq!(EscrowDiscriminator::BlockMint as u8, 7);
        assert_eq!(EscrowDiscriminator::BlockTokenExtension as u8, 8);
        assert_eq!(EscrowDiscriminator::SetArbiter as u8, 9);
    }

    #[test]
    fn test_build_escrow_instruction_no_args() {
        let accounts = vec![];
        let ix = build_escrow_instruction(EscrowDiscriminator::UpdateAdmin, accounts, vec![]);
        assert_eq!(ix.program_id, ESCROW_PROGRAM_ID);
        assert_eq!(ix.data, vec![4u8]); // discriminator only
    }

    #[test]
    fn test_build_escrow_instruction_with_args() {
        let accounts = vec![];
        let args = vec![0xAA, 0xBB];
        let ix = build_escrow_instruction(EscrowDiscriminator::Deposit, accounts, args);
        assert_eq!(ix.data, vec![3u8, 0xAA, 0xBB]); // discriminator + args
    }
}
