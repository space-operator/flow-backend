//! Solana Rewards Program Space Operator nodes
//!
//! Program ID: `REWArDioXgQJ2fZKkfu9LCLjQfRwYWVVfsvcsR5hoXi`
//! Repository: https://github.com/solana-program/rewards
//!
//! Direct instruction construction (no SDK crate dependency).
//! This is a Pinocchio-based program using single-byte discriminators.

use crate::prelude::*;
use solana_program::pubkey;

pub mod pda;

// =============================================================================
// Direct Distribution
// =============================================================================

pub mod add_direct_recipient;
pub mod claim_direct;
pub mod close_direct_distribution;
pub mod close_direct_recipient;
pub mod create_direct_distribution;
pub mod revoke_direct_recipient;

// =============================================================================
// Merkle Distribution
// =============================================================================

pub mod claim_merkle;
pub mod close_merkle_claim;
pub mod close_merkle_distribution;
pub mod create_merkle_distribution;
pub mod revoke_merkle_claim;

// =============================================================================
// Continuous Pool
// =============================================================================

pub mod claim_continuous;
pub mod claim_continuous_merkle;
pub mod close_continuous_pool;
pub mod continuous_opt_in;
pub mod continuous_opt_out;
pub mod create_continuous_pool;
pub mod distribute_continuous_reward;
pub mod revoke_continuous_user;
pub mod set_continuous_balance;
pub mod set_continuous_merkle_root;
pub mod sync_continuous_balance;

// =============================================================================
// Program Constants
// =============================================================================

/// Rewards program ID (mainnet / devnet)
pub const REWARDS_PROGRAM_ID: Pubkey = pubkey!("REWArDioXgQJ2fZKkfu9LCLjQfRwYWVVfsvcsR5hoXi");

/// Default token program (SPL Token)
pub const DEFAULT_TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Serde default: returns the SPL Token program ID
pub fn default_token_program() -> Pubkey {
    DEFAULT_TOKEN_PROGRAM
}

// =============================================================================
// Discriminator
// =============================================================================

/// Rewards instruction discriminators (single byte, Pinocchio-style)
#[repr(u8)]
pub enum RewardsDiscriminator {
    CreateDirectDistribution = 0,
    AddDirectRecipient = 1,
    ClaimDirect = 2,
    CloseDirectDistribution = 3,
    CloseDirectRecipient = 4,
    CreateMerkleDistribution = 5,
    ClaimMerkle = 6,
    CloseMerkleClaim = 7,
    CloseMerkleDistribution = 8,
    RevokeDirectRecipient = 9,
    RevokeMerkleClaim = 10,
    CreateContinuousPool = 11,
    ContinuousOptIn = 12,
    ContinuousOptOut = 13,
    DistributeContinuousReward = 14,
    ClaimContinuous = 15,
    SyncContinuousBalance = 16,
    SetContinuousBalance = 17,
    CloseContinuousPool = 18,
    RevokeContinuousUser = 19,
    SetContinuousMerkleRoot = 20,
    ClaimContinuousMerkle = 21,
}

/// Build a rewards instruction: 1-byte discriminator + args data.
pub fn build_rewards_instruction(
    discriminator: RewardsDiscriminator,
    accounts: Vec<solana_program::instruction::AccountMeta>,
    args_data: Vec<u8>,
) -> solana_program::instruction::Instruction {
    let mut data = Vec::with_capacity(1 + args_data.len());
    data.push(discriminator as u8);
    data.extend_from_slice(&args_data);
    solana_program::instruction::Instruction {
        program_id: REWARDS_PROGRAM_ID,
        accounts,
        data,
    }
}

// =============================================================================
// Custom Borsh Types
// =============================================================================

/// Vesting schedule for direct and merkle distributions.
///
/// Serialized as: u8 tag + variant fields (Borsh LE).
/// - 0: Immediate (no extra fields)
/// - 1: Linear { start_ts: i64, end_ts: i64 }
/// - 2: Cliff { cliff_ts: i64 }
/// - 3: CliffLinear { start_ts: i64, cliff_ts: i64, end_ts: i64 }
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum VestingSchedule {
    Immediate,
    Linear {
        start_ts: i64,
        end_ts: i64,
    },
    Cliff {
        cliff_ts: i64,
    },
    CliffLinear {
        start_ts: i64,
        cliff_ts: i64,
        end_ts: i64,
    },
}

impl borsh::BorshSerialize for VestingSchedule {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            VestingSchedule::Immediate => {
                writer.write_all(&[0u8])?;
            }
            VestingSchedule::Linear { start_ts, end_ts } => {
                writer.write_all(&[1u8])?;
                writer.write_all(&start_ts.to_le_bytes())?;
                writer.write_all(&end_ts.to_le_bytes())?;
            }
            VestingSchedule::Cliff { cliff_ts } => {
                writer.write_all(&[2u8])?;
                writer.write_all(&cliff_ts.to_le_bytes())?;
            }
            VestingSchedule::CliffLinear {
                start_ts,
                cliff_ts,
                end_ts,
            } => {
                writer.write_all(&[3u8])?;
                writer.write_all(&start_ts.to_le_bytes())?;
                writer.write_all(&cliff_ts.to_le_bytes())?;
                writer.write_all(&end_ts.to_le_bytes())?;
            }
        }
        Ok(())
    }
}

/// Revocation mode for revoking distributions.
///
/// - NonVested (0): Vested tokens go to user, unvested to authority
/// - Full (1): All tokens returned to authority
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RevokeMode {
    NonVested,
    Full,
}

impl borsh::BorshSerialize for RevokeMode {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            RevokeMode::NonVested => writer.write_all(&[0u8]),
            RevokeMode::Full => writer.write_all(&[1u8]),
        }
    }
}

/// Balance source for continuous reward pools.
///
/// - OnChain (0): Balance read from on-chain token account
/// - AuthoritySet (1): Balance set manually by authority
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BalanceSource {
    OnChain,
    AuthoritySet,
}

impl borsh::BorshSerialize for BalanceSource {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            BalanceSource::OnChain => writer.write_all(&[0u8]),
            BalanceSource::AuthoritySet => writer.write_all(&[1u8]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_values() {
        assert_eq!(RewardsDiscriminator::CreateDirectDistribution as u8, 0);
        assert_eq!(RewardsDiscriminator::AddDirectRecipient as u8, 1);
        assert_eq!(RewardsDiscriminator::ClaimDirect as u8, 2);
        assert_eq!(RewardsDiscriminator::CloseDirectDistribution as u8, 3);
        assert_eq!(RewardsDiscriminator::CloseDirectRecipient as u8, 4);
        assert_eq!(RewardsDiscriminator::CreateMerkleDistribution as u8, 5);
        assert_eq!(RewardsDiscriminator::ClaimMerkle as u8, 6);
        assert_eq!(RewardsDiscriminator::CloseMerkleClaim as u8, 7);
        assert_eq!(RewardsDiscriminator::CloseMerkleDistribution as u8, 8);
        assert_eq!(RewardsDiscriminator::RevokeDirectRecipient as u8, 9);
        assert_eq!(RewardsDiscriminator::RevokeMerkleClaim as u8, 10);
        assert_eq!(RewardsDiscriminator::CreateContinuousPool as u8, 11);
        assert_eq!(RewardsDiscriminator::ContinuousOptIn as u8, 12);
        assert_eq!(RewardsDiscriminator::ContinuousOptOut as u8, 13);
        assert_eq!(RewardsDiscriminator::DistributeContinuousReward as u8, 14);
        assert_eq!(RewardsDiscriminator::ClaimContinuous as u8, 15);
        assert_eq!(RewardsDiscriminator::SyncContinuousBalance as u8, 16);
        assert_eq!(RewardsDiscriminator::SetContinuousBalance as u8, 17);
        assert_eq!(RewardsDiscriminator::CloseContinuousPool as u8, 18);
        assert_eq!(RewardsDiscriminator::RevokeContinuousUser as u8, 19);
        assert_eq!(RewardsDiscriminator::SetContinuousMerkleRoot as u8, 20);
        assert_eq!(RewardsDiscriminator::ClaimContinuousMerkle as u8, 21);
    }

    #[test]
    fn test_build_rewards_instruction_no_args() {
        let ix = build_rewards_instruction(
            RewardsDiscriminator::CloseDirectDistribution,
            vec![],
            vec![],
        );
        assert_eq!(ix.program_id, REWARDS_PROGRAM_ID);
        assert_eq!(ix.data, vec![3u8]);
    }

    #[test]
    fn test_build_rewards_instruction_with_args() {
        let args = vec![0xAA, 0xBB];
        let ix = build_rewards_instruction(RewardsDiscriminator::ClaimDirect, vec![], args);
        assert_eq!(ix.data, vec![2u8, 0xAA, 0xBB]);
    }

    #[test]
    fn test_vesting_schedule_borsh_immediate() {
        let schedule = VestingSchedule::Immediate;
        let bytes = borsh::to_vec(&schedule).unwrap();
        assert_eq!(bytes, vec![0u8]);
    }

    #[test]
    fn test_vesting_schedule_borsh_linear() {
        let schedule = VestingSchedule::Linear {
            start_ts: 1000,
            end_ts: 2000,
        };
        let bytes = borsh::to_vec(&schedule).unwrap();
        assert_eq!(bytes[0], 1u8); // u8 tag, not u32
        assert_eq!(bytes.len(), 1 + 8 + 8); // tag + start_ts + end_ts
    }

    #[test]
    fn test_vesting_schedule_borsh_cliff() {
        let schedule = VestingSchedule::Cliff { cliff_ts: 5000 };
        let bytes = borsh::to_vec(&schedule).unwrap();
        assert_eq!(bytes[0], 2u8);
        assert_eq!(bytes.len(), 1 + 8);
    }

    #[test]
    fn test_vesting_schedule_borsh_cliff_linear() {
        let schedule = VestingSchedule::CliffLinear {
            start_ts: 1000,
            cliff_ts: 1500,
            end_ts: 2000,
        };
        let bytes = borsh::to_vec(&schedule).unwrap();
        assert_eq!(bytes[0], 3u8);
        assert_eq!(bytes.len(), 1 + 8 + 8 + 8);
    }

    #[test]
    fn test_revoke_mode_borsh() {
        assert_eq!(borsh::to_vec(&RevokeMode::NonVested).unwrap(), vec![0u8]);
        assert_eq!(borsh::to_vec(&RevokeMode::Full).unwrap(), vec![1u8]);
    }

    #[test]
    fn test_balance_source_borsh() {
        assert_eq!(borsh::to_vec(&BalanceSource::OnChain).unwrap(), vec![0u8]);
        assert_eq!(
            borsh::to_vec(&BalanceSource::AuthoritySet).unwrap(),
            vec![1u8]
        );
    }

    #[test]
    fn test_vesting_schedule_json_roundtrip() {
        let schedule = VestingSchedule::Linear {
            start_ts: 1000,
            end_ts: 2000,
        };
        let json = serde_json::to_value(&schedule).unwrap();
        let parsed: VestingSchedule = serde_json::from_value(json).unwrap();
        let bytes1 = borsh::to_vec(&schedule).unwrap();
        let bytes2 = borsh::to_vec(&parsed).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_revoke_mode_json_roundtrip() {
        let mode = RevokeMode::Full;
        let json = serde_json::to_value(&mode).unwrap();
        let parsed: RevokeMode = serde_json::from_value(json).unwrap();
        assert_eq!(
            borsh::to_vec(&mode).unwrap(),
            borsh::to_vec(&parsed).unwrap()
        );
    }
}
