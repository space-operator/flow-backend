//! SPL Governance v3.1.2 - Space Operator nodes
//!
//! Program ID: `GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw`
//! Upstream: https://github.com/Mythic-Project/spl-governance-v3.1.2

pub mod prelude {
    pub use flow_lib::command::prelude::*;
    pub use flow_lib::solana::Wallet;
    pub use solana_program::instruction::Instruction;
}

use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

// Existing instructions
pub mod add_signatory;
pub mod cancel_proposal;
pub mod cast_vote;
pub mod complete_proposal;
pub mod create_governance;
pub mod create_native_treasury;
pub mod create_proposal;
pub mod create_realm;
pub mod create_token_owner_record;
pub mod deposit_governing_tokens;
pub mod execute_transaction;
pub mod finalize_vote;
pub mod insert_transaction;
pub mod refund_proposal_deposit;
pub mod relinquish_vote;
pub mod remove_transaction;
pub mod revoke_governing_tokens;
pub mod set_governance_config;
pub mod set_governance_delegate;
pub mod set_realm_authority;
pub mod set_realm_config;
pub mod sign_off_proposal;
pub mod withdraw_governing_tokens;
// CHAT
pub mod post_message;
// NEW in v3.1.2
pub mod close_transaction_buffer;
pub mod create_transaction_buffer;
pub mod execute_versioned_transaction;
pub mod extend_transaction_buffer;
pub mod flag_transaction_error;
pub mod insert_versioned_transaction;
pub mod insert_versioned_transaction_from_buffer;
pub mod remove_versioned_transaction;
pub mod update_program_metadata;

pub const SPL_GOVERNANCE_ID: Pubkey =
    Pubkey::from_str_const("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw");
pub const SPL_GOVERNANCE_CHAT_ID: Pubkey =
    Pubkey::from_str_const("gCHAtYKrUUktTVzE4hEnZdLV4LXrdBf6Hh9qMaJALET");
/// Returns the appropriate token program ID based on whether the token is Token-2022.
pub fn spl_token_program_id(is_token_2022: bool) -> Pubkey {
    if is_token_2022 {
        spl_token_2022_interface::ID
    } else {
        spl_token_interface::ID
    }
}

/// Instructions supported by the GovernanceChat program
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceChatInstruction {
    /// Posts a message with a comment for a Proposal
    PostMessage {
        #[allow(dead_code)]
        body: MessageBody,
        #[allow(dead_code)]
        is_reply: bool,
    },
}

/// Chat message body
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum MessageBody {
    Text(String),
    Reaction(String),
}

/// Instructions supported by the Governance program
/// Enum variant order matches v3.1.2 borsh discriminants exactly.
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    // === Index 0 ===
    CreateRealm {
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        config_args: RealmConfigArgs,
    },

    // === Index 1 ===
    DepositGoverningTokens {
        #[allow(dead_code)]
        amount: u64,
    },

    // === Index 2 ===
    WithdrawGoverningTokens {},

    // === Index 3 ===
    SetGovernanceDelegate {
        #[allow(dead_code)]
        new_governance_delegate: Option<Pubkey>,
    },

    // === Index 4 ===
    CreateGovernance {
        #[allow(dead_code)]
        config: GovernanceConfig,
    },

    // === Index 5 ===
    /// Formerly CreateProgramGovernance. Exists for backwards-compatibility.
    Legacy4,

    // === Index 6 ===
    CreateProposal {
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        description_link: String,
        #[allow(dead_code)]
        vote_type: VoteType,
        #[allow(dead_code)]
        options: Vec<String>,
        #[allow(dead_code)]
        use_deny_option: bool,
        #[allow(dead_code)]
        proposal_seed: Pubkey,
    },

    // === Index 7 ===
    /// v3.1.2 accounts: [proposal(w), token_owner_record, governance_authority(s),
    /// signatory_record(w), payer(s), system]
    AddSignatory {
        #[allow(dead_code)]
        signatory: Pubkey,
    },

    // === Index 8 ===
    /// Formerly RemoveSignatory. Exists for backwards-compatibility.
    Legacy1,

    // === Index 9 ===
    InsertTransaction {
        #[allow(dead_code)]
        option_index: u8,
        #[allow(dead_code)]
        index: u16,
        #[allow(dead_code)]
        hold_up_time: u32,
        #[allow(dead_code)]
        instructions: Vec<InstructionData>,
    },

    // === Index 10 ===
    RemoveTransaction,

    // === Index 11 ===
    CancelProposal,

    // === Index 12 ===
    SignOffProposal,

    // === Index 13 ===
    CastVote {
        #[allow(dead_code)]
        vote: Vote,
    },

    // === Index 14 ===
    FinalizeVote {},

    // === Index 15 ===
    RelinquishVote,

    // === Index 16 ===
    ExecuteTransaction,

    // === Index 17 ===
    /// Formerly CreateMintGovernance. Exists for backwards-compatibility.
    Legacy2,

    // === Index 18 ===
    /// Formerly CreateTokenGovernance. Exists for backwards-compatibility.
    Legacy3,

    // === Index 19 ===
    SetGovernanceConfig {
        #[allow(dead_code)]
        config: GovernanceConfig,
    },

    // === Index 20 ===
    /// Flags a transaction and its parent proposal with error status
    FlagTransactionError {},

    // === Index 21 ===
    SetRealmAuthority {
        #[allow(dead_code)]
        action: SetRealmAuthorityAction,
    },

    // === Index 22 ===
    SetRealmConfig {
        #[allow(dead_code)]
        config_args: RealmConfigArgs,
    },

    // === Index 23 ===
    CreateTokenOwnerRecord {},

    // === Index 24 ===
    UpdateProgramMetadata {},

    // === Index 25 ===
    CreateNativeTreasury,

    // === Index 26 ===
    RevokeGoverningTokens {
        #[allow(dead_code)]
        amount: u64,
    },

    // === Index 27 ===
    RefundProposalDeposit {},

    // === Index 28 ===
    CompleteProposal {},

    // === Index 29 === NEW in v3.1.2
    CreateTransactionBuffer {
        #[allow(dead_code)]
        buffer_index: u8,
        #[allow(dead_code)]
        final_buffer_hash: [u8; 32],
        #[allow(dead_code)]
        final_buffer_size: u16,
        #[allow(dead_code)]
        buffer: Vec<u8>,
    },

    // === Index 30 === NEW in v3.1.2
    ExtendTransactionBuffer {
        #[allow(dead_code)]
        buffer_index: u8,
        #[allow(dead_code)]
        buffer: Vec<u8>,
    },

    // === Index 31 === NEW in v3.1.2
    CloseTransactionBuffer {
        #[allow(dead_code)]
        buffer_index: u8,
    },

    // === Index 32 === NEW in v3.1.2
    InsertVersionedTransactionFromBuffer {
        #[allow(dead_code)]
        option_index: u8,
        #[allow(dead_code)]
        ephemeral_signers: u8,
        #[allow(dead_code)]
        transaction_index: u16,
    },

    // === Index 33 === NEW in v3.1.2
    InsertVersionedTransaction {
        #[allow(dead_code)]
        option_index: u8,
        #[allow(dead_code)]
        ephemeral_signers: u8,
        #[allow(dead_code)]
        transaction_index: u16,
        #[allow(dead_code)]
        transaction_message: Vec<u8>,
    },

    // === Index 34 === NEW in v3.1.2
    ExecuteVersionedTransaction,

    // === Index 35 === NEW in v3.1.2
    RemoveVersionedTransaction,
}

// ─── PDA Derivation Helpers ───────────────────────────────────────────────

/// PDA: ['transaction_buffer', proposal, creator, buffer_index]
pub fn get_proposal_transaction_buffer_address(
    program_id: &Pubkey,
    proposal: &Pubkey,
    creator: &Pubkey,
    buffer_index: &[u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"transaction_buffer",
            proposal.as_ref(),
            creator.as_ref(),
            buffer_index,
        ],
        program_id,
    )
    .0
}

/// PDA: ['version_transaction', proposal, option_index, transaction_index]
pub fn get_proposal_versioned_transaction_address(
    program_id: &Pubkey,
    proposal: &Pubkey,
    option_index: &[u8],
    transaction_index: &[u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"version_transaction",
            proposal.as_ref(),
            option_index,
            transaction_index,
        ],
        program_id,
    )
    .0
}

/// PDA: ['metadata']
pub fn get_program_metadata_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"metadata"], program_id).0
}

// ─── Shared Types ─────────────────────────────────────────────────────────

/// Realm Config instruction args
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct RealmConfigArgs {
    pub use_council_mint: bool,
    pub min_community_weight_to_create_governance: u64,
    pub community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
    pub community_token_config_args: GoverningTokenConfigArgs,
    pub council_token_config_args: GoverningTokenConfigArgs,
}

/// Realm Config instruction args with account parameters
#[derive(
    Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Default, Deserialize, Serialize,
)]
pub struct GoverningTokenConfigAccountArgs {
    pub voter_weight_addin: Option<Pubkey>,
    pub max_voter_weight_addin: Option<Pubkey>,
    pub token_type: GoverningTokenType,
}

/// Realm Config instruction args
#[derive(
    Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Default, Deserialize, Serialize,
)]
pub struct GoverningTokenConfigArgs {
    pub use_voter_weight_addin: bool,
    pub use_max_voter_weight_addin: bool,
    pub token_type: GoverningTokenType,
}

#[derive(
    Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Deserialize, Serialize, Default,
)]
pub enum GoverningTokenType {
    #[default]
    Liquid,
    Membership,
    Dormant,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub enum MintMaxVoterWeightSource {
    SupplyFraction(u64),
    Absolute(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub community_vote_threshold: VoteThreshold,
    pub min_community_weight_to_create_proposal: u64,
    pub transactions_hold_up_time: u32,
    pub voting_base_time: u32,
    pub community_vote_tipping: VoteTipping,
    pub council_vote_threshold: VoteThreshold,
    pub council_veto_vote_threshold: VoteThreshold,
    pub min_council_weight_to_create_proposal: u64,
    pub council_vote_tipping: VoteTipping,
    pub community_veto_vote_threshold: VoteThreshold,
    pub voting_cool_off_time: u32,
    pub deposit_exempt_proposal_count: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum VoteTipping {
    Strict,
    Early,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum VoteThreshold {
    YesVotePercentage(u8),
    QuorumPercentage(u8),
    Disabled,
}

/// Adds realm config account and accounts referenced by the config
pub fn with_realm_config_accounts(
    program_id: &Pubkey,
    accounts: &mut Vec<AccountMeta>,
    realm: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    max_voter_weight_record: Option<Pubkey>,
) {
    let seeds = [b"realm-config", realm.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;
    accounts.push(AccountMeta::new_readonly(realm_config_address, false));

    if let Some(voter_weight_record) = voter_weight_record {
        accounts.push(AccountMeta::new_readonly(voter_weight_record, false));
    }

    if let Some(max_voter_weight_record) = max_voter_weight_record {
        accounts.push(AccountMeta::new_readonly(max_voter_weight_record, false));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum VoteType {
    SingleChoice,
    MultiChoice {
        #[allow(dead_code)]
        choice_type: MultiChoiceType,
        #[allow(dead_code)]
        min_voter_options: u8,
        #[allow(dead_code)]
        max_voter_options: u8,
        #[allow(dead_code)]
        max_winning_options: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum MultiChoiceType {
    FullWeight,
    Weighted,
}

/// In v3.1.2, AddSignatory always uses ProposalOwner authority
/// (token_owner_record + governance_authority are separate accounts).
/// Kept for backwards compatibility with existing add_signatory.rs
#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum AddSignatoryAuthority {
    ProposalOwner {
        governance_authority: Pubkey,
        token_owner_record: Pubkey,
    },
    None,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum AddSignatoryAuthoritySPO {
    ProposalOwner {
        governance_authority: String,
        token_owner_record: String,
    },
    None,
}

impl From<AddSignatoryAuthoritySPO> for AddSignatoryAuthority {
    fn from(authority: AddSignatoryAuthoritySPO) -> Self {
        match authority {
            AddSignatoryAuthoritySPO::ProposalOwner {
                governance_authority,
                token_owner_record,
            } => AddSignatoryAuthority::ProposalOwner {
                governance_authority: Pubkey::from_str(&governance_authority).unwrap(),
                token_owner_record: Pubkey::from_str(&token_owner_record).unwrap(),
            },
            AddSignatoryAuthoritySPO::None => AddSignatoryAuthority::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum Vote {
    Approve(Vec<VoteChoice>),
    Deny,
    Abstain,
    Veto,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct VoteChoice {
    pub rank: u8,
    pub weight_percentage: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct InstructionData {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMetaData>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct AccountMetaData {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<Instruction> for InstructionData {
    fn from(instruction: Instruction) -> Self {
        InstructionData {
            program_id: instruction.program_id,
            accounts: instruction
                .accounts
                .iter()
                .map(|a| AccountMetaData {
                    pubkey: a.pubkey,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect(),
            data: instruction.data,
        }
    }
}

impl From<&InstructionData> for Instruction {
    fn from(instruction: &InstructionData) -> Self {
        Instruction {
            program_id: instruction.program_id,
            accounts: instruction
                .accounts
                .iter()
                .map(|a| AccountMeta {
                    pubkey: a.pubkey,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect(),
            data: instruction.data.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum SetRealmAuthorityAction {
    SetUnchecked,
    SetChecked,
    Remove,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PDA Derivation Tests ──────────────────────────────────────────

    #[test]
    fn test_realm_pda() {
        let realm = Pubkey::find_program_address(
            &[b"governance", b"my-dao"],
            &SPL_GOVERNANCE_ID,
        );
        // Deterministic: same seeds always produce the same address
        let realm2 = Pubkey::find_program_address(
            &[b"governance", b"my-dao"],
            &SPL_GOVERNANCE_ID,
        );
        assert_eq!(realm, realm2);

        // Different name → different PDA
        let other = Pubkey::find_program_address(
            &[b"governance", b"other-dao"],
            &SPL_GOVERNANCE_ID,
        );
        assert_ne!(realm.0, other.0);
    }

    #[test]
    fn test_realm_config_pda() {
        let realm = Pubkey::new_unique();
        let config = Pubkey::find_program_address(
            &[b"realm-config", realm.as_ref()],
            &SPL_GOVERNANCE_ID,
        );
        // Same realm → same config PDA
        let config2 = Pubkey::find_program_address(
            &[b"realm-config", realm.as_ref()],
            &SPL_GOVERNANCE_ID,
        );
        assert_eq!(config, config2);
    }

    #[test]
    fn test_token_owner_record_pda() {
        let realm = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let pda = Pubkey::find_program_address(
            &[b"governance", realm.as_ref(), mint.as_ref(), owner.as_ref()],
            &SPL_GOVERNANCE_ID,
        );
        // Different owner → different PDA
        let other_owner = Pubkey::new_unique();
        let pda2 = Pubkey::find_program_address(
            &[b"governance", realm.as_ref(), mint.as_ref(), other_owner.as_ref()],
            &SPL_GOVERNANCE_ID,
        );
        assert_ne!(pda.0, pda2.0);
    }

    #[test]
    fn test_vote_record_pda() {
        let proposal = Pubkey::new_unique();
        let voter_token_owner_record = Pubkey::new_unique();
        let pda = Pubkey::find_program_address(
            &[b"governance", proposal.as_ref(), voter_token_owner_record.as_ref()],
            &SPL_GOVERNANCE_ID,
        );
        assert_ne!(pda.0, Pubkey::default());
    }

    #[test]
    fn test_transaction_buffer_pda() {
        let proposal = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let addr = get_proposal_transaction_buffer_address(
            &SPL_GOVERNANCE_ID,
            &proposal,
            &creator,
            &[0],
        );
        // Different buffer index → different PDA
        let addr2 = get_proposal_transaction_buffer_address(
            &SPL_GOVERNANCE_ID,
            &proposal,
            &creator,
            &[1],
        );
        assert_ne!(addr, addr2);
    }

    #[test]
    fn test_versioned_transaction_pda() {
        let proposal = Pubkey::new_unique();
        let addr = get_proposal_versioned_transaction_address(
            &SPL_GOVERNANCE_ID,
            &proposal,
            &[0],
            &[0],
        );
        assert_ne!(addr, Pubkey::default());
    }

    #[test]
    fn test_program_metadata_pda() {
        let addr = get_program_metadata_address(&SPL_GOVERNANCE_ID);
        let addr2 = get_program_metadata_address(&SPL_GOVERNANCE_ID);
        assert_eq!(addr, addr2);
    }

    // ── Borsh Serialization Roundtrip Tests ───────────────────────────

    #[test]
    fn test_borsh_create_realm() {
        let ix = GovernanceInstruction::CreateRealm {
            name: "test-dao".to_string(),
            config_args: RealmConfigArgs {
                use_council_mint: false,
                min_community_weight_to_create_governance: 1000,
                community_mint_max_voter_weight_source: MintMaxVoterWeightSource::SupplyFraction(
                    10_000_000_000,
                ),
                community_token_config_args: GoverningTokenConfigArgs::default(),
                council_token_config_args: GoverningTokenConfigArgs::default(),
            },
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert!(!bytes.is_empty());
        // Discriminant for CreateRealm is 0
        assert_eq!(bytes[0], 0);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_cast_vote_approve() {
        let ix = GovernanceInstruction::CastVote {
            vote: Vote::Approve(vec![VoteChoice {
                rank: 0,
                weight_percentage: 100,
            }]),
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        // Discriminant for CastVote is 13
        assert_eq!(bytes[0], 13);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_cast_vote_deny() {
        let ix = GovernanceInstruction::CastVote { vote: Vote::Deny };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 13);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_deposit_governing_tokens() {
        let ix = GovernanceInstruction::DepositGoverningTokens { amount: 1_000_000 };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 1);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_create_governance() {
        let ix = GovernanceInstruction::CreateGovernance {
            config: GovernanceConfig {
                community_vote_threshold: VoteThreshold::YesVotePercentage(60),
                min_community_weight_to_create_proposal: 100,
                transactions_hold_up_time: 0,
                voting_base_time: 259200,
                community_vote_tipping: VoteTipping::Early,
                council_vote_threshold: VoteThreshold::YesVotePercentage(60),
                council_veto_vote_threshold: VoteThreshold::Disabled,
                min_council_weight_to_create_proposal: 1,
                council_vote_tipping: VoteTipping::Early,
                community_veto_vote_threshold: VoteThreshold::Disabled,
                voting_cool_off_time: 43200,
                deposit_exempt_proposal_count: 10,
            },
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 4);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_create_proposal() {
        let ix = GovernanceInstruction::CreateProposal {
            name: "Test Proposal".to_string(),
            description_link: "https://example.com".to_string(),
            vote_type: VoteType::SingleChoice,
            options: vec!["Yes".to_string()],
            use_deny_option: true,
            proposal_seed: Pubkey::new_unique(),
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 6);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_insert_transaction() {
        let ix = GovernanceInstruction::InsertTransaction {
            option_index: 0,
            index: 0,
            hold_up_time: 0,
            instructions: vec![],
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 9);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_set_realm_authority() {
        let ix = GovernanceInstruction::SetRealmAuthority {
            action: SetRealmAuthorityAction::SetChecked,
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 21);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    #[test]
    fn test_borsh_v312_create_transaction_buffer() {
        let ix = GovernanceInstruction::CreateTransactionBuffer {
            buffer_index: 0,
            final_buffer_hash: [0u8; 32],
            final_buffer_size: 100,
            buffer: vec![1, 2, 3],
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        assert_eq!(bytes[0], 29);
        let decoded: GovernanceInstruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, decoded);
    }

    // ── Shared Type Tests ─────────────────────────────────────────────

    #[test]
    fn test_instruction_data_roundtrip() {
        let ix = Instruction {
            program_id: SPL_GOVERNANCE_ID,
            accounts: vec![AccountMeta::new(Pubkey::new_unique(), true)],
            data: vec![1, 2, 3],
        };
        let ix_data: InstructionData = ix.clone().into();
        let ix_back: Instruction = (&ix_data).into();
        assert_eq!(ix.program_id, ix_back.program_id);
        assert_eq!(ix.data, ix_back.data);
        assert_eq!(ix.accounts.len(), ix_back.accounts.len());
    }

    #[test]
    fn test_spl_token_program_id_selector() {
        assert_eq!(spl_token_program_id(false), spl_token_interface::ID);
        assert_eq!(spl_token_program_id(true), spl_token_2022_interface::ID);
    }
}
