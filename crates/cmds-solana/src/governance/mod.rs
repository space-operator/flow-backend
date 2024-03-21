use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    clock::UnixTimestamp,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

pub mod add_required_signatory;
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
pub mod relinquish_token_owner_record_locks;
pub mod relinquish_vote;
pub mod remove_transaction;
pub mod revoke_governing_tokens;
pub mod set_governance_config;
pub mod set_governance_delegate;
pub mod set_realm_authority;
pub mod set_realm_config;
pub mod set_token_owner_record_lock;
pub mod sign_off_proposal;
pub mod withdraw_governing_tokens;

const SPL_GOVERNANCE_ID: &str = "GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw";

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance Realm account which aggregates governances for given
    CreateRealm {
        #[allow(dead_code)]
        /// UTF-8 encoded Governance Realm name
        name: String,

        #[allow(dead_code)]
        /// Realm config args
        config_args: RealmConfigArgs,
    },

    /// Deposits governing tokens (Community or Council) to Governance Realm and
    /// establishes your voter weight to be used for voting within the Realm
    /// Note: If subsequent (top up) deposit is made and there are active votes
    /// for the Voter then the vote weights won't be updated automatically
    /// It can be done by relinquishing votes on active Proposals and voting
    /// again with the new weight
    DepositGoverningTokens {
        /// The amount to deposit into the realm
        #[allow(dead_code)]
        amount: u64,
    },

    /// Withdraws governing tokens (Community or Council) from Governance Realm
    /// and downgrades your voter weight within the Realm.
    /// Note: It's only possible to withdraw tokens if the Voter doesn't have
    /// any outstanding active votes.
    /// If there are any outstanding votes then they must be relinquished
    /// before tokens could be withdrawn
    WithdrawGoverningTokens {},

    /// Sets Governance Delegate for the given Realm and Governing Token Mint
    /// (Community or Council). The Delegate would have voting rights and
    /// could vote on behalf of the Governing Token Owner. The Delegate would
    /// also be able to create Proposals on behalf of the Governing Token
    /// Owner.
    /// Note: This doesn't take voting rights from the Token Owner who still can
    /// vote and change governance_delegate
    SetGovernanceDelegate {
        #[allow(dead_code)]
        /// New Governance Delegate
        new_governance_delegate: Option<Pubkey>,
    },

    /// Creates Governance account which can be used to govern any arbitrary
    /// Solana account or asset
    CreateGovernance {
        /// Governance config
        #[allow(dead_code)]
        config: GovernanceConfig,
    },

    /// Legacy CreateProgramGovernance instruction
    /// Exists for backwards-compatibility
    Legacy4,

    /// Creates Proposal account for Transactions which will be executed at some
    /// point in the future
    ///
    ///   0. `[]` Realm account the created Proposal belongs to
    ///   1. `[writable]` Proposal account.
    ///     * PDA seeds ['governance',governance, governing_token_mint,
    ///       proposal_seed]
    ///   2. `[writable]` Governance account
    ///   3. `[writable]` TokenOwnerRecord account of the Proposal owner
    ///   4. `[]` Governing Token Mint the Proposal is created for
    ///   5. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   6. `[signer]` Payer
    ///   7. `[]` System program
    ///   8. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   9. `[]` Optional Voter Weight Record
    ///   10.`[writable]` Optional ProposalDeposit account.
    ///     * PDA seeds: ['proposal-deposit', proposal, deposit payer]
    ///     Proposal deposit is required when there are more active proposals
    ///     than the configured deposit exempt amount.
    ///     The deposit is paid by the Payer of the transaction and can be
    ///     reclaimed using RefundProposalDeposit once the Proposal is no
    ///     longer active.
    CreateProposal {
        #[allow(dead_code)]
        /// UTF-8 encoded name of the proposal
        name: String,

        #[allow(dead_code)]
        /// Link to a gist explaining the proposal
        description_link: String,

        #[allow(dead_code)]
        /// Proposal vote type
        vote_type: VoteType,

        #[allow(dead_code)]
        /// Proposal options
        options: Vec<String>,

        #[allow(dead_code)]
        /// Indicates whether the proposal has the deny option
        /// A proposal without the rejecting option is a non binding survey
        /// Only proposals with the rejecting option can have executable
        /// transactions
        use_deny_option: bool,

        #[allow(dead_code)]
        /// Unique seed for the Proposal PDA
        proposal_seed: Pubkey,
    },

    /// Adds a signatory to the Proposal which means this Proposal can't leave
    /// Draft state until yet another Signatory signs
    ///
    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account associated with the governance
    ///   2. `[writable]` Signatory Record Account
    ///   3. `[signer]` Payer
    ///   4. `[]` System program
    ///   Either:
    ///      - 5. `[]` TokenOwnerRecord account of the Proposal owner
    ///        6. `[signer]` Governance Authority (Token Owner or Governance
    ///           Delegate)
    ///
    ///      - 5. `[]` RequiredSignatory account associated with the governance.
    AddSignatory {
        #[allow(dead_code)]
        /// Signatory to add to the Proposal
        signatory: Pubkey,
    },

    /// Formerly RemoveSignatory. Exists for backwards-compatibility.
    Legacy1,

    /// Inserts Transaction with a set of instructions for the Proposal at the
    /// given index position New Transaction must be inserted at the end of
    /// the range indicated by Proposal transactions_next_index
    /// If a Transaction replaces an existing Transaction at a given index then
    /// the old one must be removed using RemoveTransaction first

    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[]` TokenOwnerRecord account of the Proposal owner
    ///   3. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   4. `[writable]` ProposalTransaction, account.
    ///     * PDA seeds: ['governance', proposal, option_index, index]
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Rent sysvar
    InsertTransaction {
        #[allow(dead_code)]
        /// The index of the option the transaction is for
        option_index: u8,
        #[allow(dead_code)]
        /// Transaction index to be inserted at.
        index: u16,
        #[allow(dead_code)]
        /// Legacy hold_up_time
        legacy: u32,

        #[allow(dead_code)]
        /// Instructions Data
        instructions: Vec<InstructionData>,
    },

    /// Removes Transaction from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   3. `[writable]` ProposalTransaction, account
    ///   4. `[writable]` Beneficiary Account which would receive lamports from
    ///      the disposed ProposalTransaction account
    RemoveTransaction,

    /// Cancels Proposal by changing its state to Canceled
    ///
    ///   0. `[]` Realm account
    ///   1. `[writable]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]`  TokenOwnerRecord account of the  Proposal owner
    ///   4. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    CancelProposal,

    /// Signs off Proposal indicating the Signatory approves the Proposal
    /// When the last Signatory signs off the Proposal it enters Voting state
    /// Note: Adding signatories to a Proposal is a quality and not a security
    /// gate and it's entirely at the discretion of the Proposal owner
    /// If Proposal owner doesn't designate any signatories then can sign off
    /// the Proposal themself
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[signer]` Signatory account signing off the Proposal Or Proposal
    ///      owner if the owner hasn't appointed any signatories
    ///   4. `[]` TokenOwnerRecord for the Proposal owner, required when the
    ///      owner signs off the Proposal Or `[writable]` SignatoryRecord
    ///      account, required when non owner sings off the Proposal
    SignOffProposal,

    ///  Uses your voter weight (deposited Community or Council tokens) to cast
    /// a vote on a Proposal  By doing so you indicate you approve or
    /// disapprove of running the Proposal set of transactions  If you tip
    /// the consensus then the transactions can begin to be run after their hold
    /// up time
    ///
    ///   0. `[]` Realm account
    ///   1. `[writable]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord of the Proposal owner
    ///   4. `[writable]` TokenOwnerRecord of the voter.
    ///     * PDA seeds: ['governance',realm, vote_governing_token_mint,
    ///       governing_token_owner]
    ///   5. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   6. `[writable]` Proposal VoteRecord account.
    ///     * PDA seeds: ['governance',proposal,token_owner_record]
    ///   7. `[]` The Governing Token Mint which is used to cast the vote
    ///      (vote_governing_token_mint).
    ///     The voting token mint is the governing_token_mint of the Proposal
    ///     for Approve, Deny and Abstain votes.
    ///     For Veto vote the voting token mint is the mint of the opposite
    ///     voting population Council mint to veto Community proposals and
    ///     Community mint to veto Council proposals.
    ///     Note: In the current version only Council veto is supported
    ///   8. `[signer]` Payer
    ///   9. `[]` System program
    ///   10. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   11. `[]` Optional Voter Weight Record
    ///   12. `[]` Optional Max Voter Weight Record
    CastVote {
        #[allow(dead_code)]
        /// User's vote
        vote: Vote,
    },

    /// Finalizes vote in case the Vote was not automatically tipped within
    /// max_voting_time period
    ///
    ///   0. `[]` Realm account
    ///   1. `[writable]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord of the Proposal owner
    ///   4. `[]` Governing Token Mint
    ///   5. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   6. `[]` Optional Max Voter Weight Record
    FinalizeVote {},

    ///  Relinquish Vote removes voter weight from a Proposal and removes it
    /// from voter's active votes. If the Proposal is still being voted on
    /// then the voter's weight won't count towards the vote outcome. If the
    /// Proposal is already in decided state then the instruction has no impact
    /// on the Proposal and only allows voters to prune their outstanding
    /// votes in case they wanted to withdraw Governing tokens from the Realm
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, vote_governing_token_mint,
    ///       governing_token_owner]
    ///   4. `[writable]` Proposal VoteRecord account.
    ///     * PDA seeds: ['governance',proposal, token_owner_record]
    ///   5. `[]` The Governing Token Mint which was used to cast the vote
    ///      (vote_governing_token_mint)
    ///   6. `[signer]` Optional Governance Authority (Token Owner or Governance
    ///      Delegate) It's required only when Proposal is still being voted on
    ///   7. `[writable]` Optional Beneficiary account which would receive
    ///      lamports when VoteRecord Account is disposed It's required only
    ///      when Proposal is still being voted on
    RelinquishVote,

    /// Executes a Transaction in the Proposal
    /// Anybody can execute transaction once Proposal has been voted Yes and
    /// transaction_hold_up time has passed The actual transaction being
    /// executed will be signed by Governance PDA the Proposal belongs to
    /// For example to execute Program upgrade the ProgramGovernance PDA would
    /// be used as the signer
    ///
    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[writable]` ProposalTransaction account you wish to execute
    ///   3+ Any extra accounts that are part of the transaction, in order
    ExecuteTransaction,

    /// Legacy CreateMintGovernance instruction
    /// Exists for backwards-compatibility
    Legacy2,

    /// Legacy CreateTokenGovernance instruction
    /// Exists for backwards-compatibility
    Legacy3,

    /// Sets GovernanceConfig for a Governance
    ///
    ///   0. `[]` Realm account the Governance account belongs to
    ///   1. `[writable, signer]` The Governance account the config is for
    SetGovernanceConfig {
        #[allow(dead_code)]
        /// New governance config
        config: GovernanceConfig,
    },

    /// Legacy FlagTransactionError instruction
    /// Exists for backwards-compatibility
    Legacy5,

    /// Sets new Realm authority
    ///
    ///   0. `[writable]` Realm account
    ///   1. `[signer]` Current Realm authority
    ///   2. `[]` New realm authority. Must be one of the realm governances when
    ///      set
    SetRealmAuthority {
        #[allow(dead_code)]
        /// Set action ( SetUnchecked, SetChecked, Remove)
        action: SetRealmAuthorityAction,
    },

    /// Sets realm config
    ///   0. `[writable]` Realm account
    ///   1. `[signer]`  Realm authority
    ///   2. `[]` Council Token Mint - optional
    ///     Note: In the current version it's only possible to remove council
    ///     mint (set it to None).
    ///     After setting council to None it won't be possible to withdraw the
    ///     tokens from the Realm any longer.
    ///     If that's required then it must be done before executing this
    ///     instruction.
    ///   3. `[writable]` Council Token Holding account - optional unless
    ///     council is used.
    ///     * PDA seeds: ['governance',realm,council_mint] The account will be
    ///     created with the Realm PDA as its owner
    ///   4. `[]` System
    ///   5. `[writable]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   6. `[]` Optional Community Voter Weight Addin Program Id
    ///   7. `[]` Optional Max Community Voter Weight Addin Program Id
    ///   8. `[]` Optional Council Voter Weight Addin Program Id
    ///   9. `[]` Optional Max Council Voter Weight Addin Program Id
    ///   10. `[signer]` Optional Payer. Required if RealmConfig doesn't exist
    ///       and needs to be created
    SetRealmConfig {
        #[allow(dead_code)]
        /// Realm config args
        config_args: RealmConfigArgs,
    },

    /// Creates TokenOwnerRecord with 0 deposit amount
    /// It's used to register TokenOwner when voter weight addin is used and the
    /// Governance program doesn't take deposits
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governing Token Owner account
    ///   2. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint,
    ///       governing_token_owner]
    ///   3. `[]` Governing Token Mint
    ///   4. `[signer]` Payer
    ///   5. `[]` System
    CreateTokenOwnerRecord {},

    /// Updates ProgramMetadata account
    /// The instruction dumps information implied by the program's code into a
    /// persistent account
    ///
    ///  0. `[writable]` ProgramMetadata account.
    ///     * PDA seeds: ['metadata']
    ///  1. `[signer]` Payer
    ///  2. `[]` System
    UpdateProgramMetadata {},

    /// Creates native SOL treasury account for a Governance account
    /// The account has no data and can be used as a payer for instructions
    /// signed by Governance PDAs or as a native SOL treasury
    ///
    ///  0. `[]` Governance account the treasury account is for
    ///  1. `[writable]` NativeTreasury account.
    ///     * PDA seeds: ['native-treasury', governance]
    ///  2. `[signer]` Payer
    ///  3. `[]` System
    CreateNativeTreasury,

    /// Revokes (burns) membership governing tokens for the given
    /// TokenOwnerRecord and hence takes away governance power from the
    /// TokenOwner. Note: If there are active votes for the TokenOwner then
    /// the vote weights won't be updated automatically
    ///
    ///  0. `[]` Realm account
    ///  1. `[writable]` Governing Token Holding account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint,
    ///       governing_token_owner]
    ///  3. `[writable]` GoverningTokenMint
    ///  4. `[signer]` Revoke authority which can be either of:
    ///                1) GoverningTokenMint mint_authority to forcefully revoke
    ///                   the membership tokens
    ///                2) GoverningTokenOwner who voluntarily revokes their own
    ///                   membership
    ///  5. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///  6. `[]` SPL Token program
    RevokeGoverningTokens {
        /// The amount to revoke
        #[allow(dead_code)]
        amount: u64,
    },

    /// Refunds ProposalDeposit once the given proposal is no longer active
    /// (Draft, SigningOff, Voting) Once the condition is met the
    /// instruction is permissionless and returns the deposit amount to the
    /// deposit payer
    ///
    ///   0. `[]` Proposal account
    ///   1. `[writable]` ProposalDeposit account.
    ///     * PDA seeds: ['proposal-deposit', proposal, deposit payer]
    ///   2. `[writable]` Proposal deposit payer (beneficiary) account
    RefundProposalDeposit {},

    /// Transitions an off-chain or manually executable Proposal from Succeeded
    /// into Completed state
    ///
    /// Upon a successful vote on an off-chain or manually executable proposal
    /// it remains in Succeeded state Once the external actions are executed
    /// the Proposal owner can use the instruction to manually transition it to
    /// Completed state
    ///
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` CompleteProposal authority (Token Owner or Delegate)
    CompleteProposal {},

    /// Adds a required signatory to the Governance, which will be applied to
    /// all proposals created with it
    ///
    ///   0. `[writable, signer]` The Governance account the config is for
    ///   1. `[writable]` RequiredSignatory Account
    ///   2. `[signer]` Payer
    ///   3. `[]` System program
    AddRequiredSignatory {
        #[allow(dead_code)]
        /// Required signatory to add to the Governance
        signatory: Pubkey,
    },

    /// Removes a required signatory from the Governance
    ///
    ///  0. `[writable, signer]` The Governance account the config is for
    ///  1. `[writable]` RequiredSignatory Account
    ///  2. `[writable]` Beneficiary Account which would receive lamports from
    ///     the disposed RequiredSignatory Account
    RemoveRequiredSignatory,

    /// Sets TokenOwnerRecord lock for the given authority and lock id
    ///
    ///   0. `[]` Realm
    ///   1. `[]` RealmConfig
    ///   2. `[writable]` TokenOwnerRecord the lock is set for
    ///   3. `[signer]` Lock authority issuing the lock
    ///   4. `[signer]` Payer
    ///   5. `[]` System
    SetTokenOwnerRecordLock {
        /// Custom lock id which can be used by the authority to issue
        /// different locks
        #[allow(dead_code)]
        lock_id: u8,

        /// The timestamp when the lock expires or None if it never expires
        #[allow(dead_code)]
        expiry: Option<UnixTimestamp>,
    },

    /// Removes all expired TokenOwnerRecord locks and if specified
    /// the locks identified by the given lock ids and authority
    ///
    ///
    ///   0. `[]` Realm
    ///   1. `[]` RealmConfig
    ///   2. `[writable]` TokenOwnerRecord the locks are removed from
    ///   3. `[signer]` Optional lock authority which issued the locks specified
    ///      by lock_ids. If the authority is configured in RealmConfig then it
    ///      must sign the transaction. If the authority is no longer configured
    ///      then the locks are removed without the authority signature
    RelinquishTokenOwnerRecordLocks {
        /// Custom lock ids identifying the lock to remove
        /// If the lock_id is None then only expired locks are removed
        #[allow(dead_code)]
        lock_ids: Option<Vec<u8>>,
    },
    // Sets Realm config item
    // Note:
    // This instruction is used to set a single RealmConfig item at a time
    // In the current version it only supports TokenOwnerRecordLockAuthority
    // however eventually all Realm configuration items should be set using
    // this instruction and SetRealmConfig instruction should be deprecated
    //
    //   0. `[writable]` Realm account
    //   1. `[writable]` RealmConfig account
    //   2. `[signer]`  Realm authority
    //   3. `[signer]` Payer
    //   4. `[]` System
    SetRealmConfigItem {
        #[allow(dead_code)]
        /// Config args
        args: SetRealmConfigItemArgs,
    },
}

/// Realm Config instruction args
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct RealmConfigArgs {
    /// Indicates whether council_mint should be used
    /// If yes then council_mint account must also be passed to the instruction
    pub use_council_mint: bool,

    /// Min number of community tokens required to create a governance
    pub min_community_weight_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_voter_weight_source: MintMaxVoterWeightSource,

    /// Community token config args
    pub community_token_config_args: GoverningTokenConfigArgs,

    /// Council token config args
    pub council_token_config_args: GoverningTokenConfigArgs,
}

/// Realm Config instruction args with account parameters
#[derive(
    Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Default, Deserialize, Serialize,
)]
pub struct GoverningTokenConfigAccountArgs {
    /// Specifies an external plugin program which should be used to provide
    /// voters weights for the given governing token
    pub voter_weight_addin: Option<Pubkey>,

    /// Specifies an external an external plugin program should be used to
    /// provide max voters weight for the given governing token
    pub max_voter_weight_addin: Option<Pubkey>,

    /// Governing token type defines how the token is used for governance power
    pub token_type: GoverningTokenType,
}

/// Realm Config instruction args
#[derive(
    Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Default, Deserialize, Serialize,
)]
pub struct GoverningTokenConfigArgs {
    /// Indicates whether an external addin program should be used to provide
    /// voters weights If yes then the voters weight program account must be
    /// passed to the instruction
    pub use_voter_weight_addin: bool,

    /// Indicates whether an external addin program should be used to provide
    /// max voters weight for the token If yes then the max voter weight
    /// program account must be passed to the instruction
    pub use_max_voter_weight_addin: bool,

    /// Governing token type defines how the token is used for governance
    pub token_type: GoverningTokenType,
}

/// The type of the governing token defines:
/// 1) Who retains the authority over deposited tokens
/// 2) Which token instructions Deposit, Withdraw and Revoke (burn) are allowed
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub enum GoverningTokenType {
    /// Liquid token is a token which is fully liquid and the token owner
    /// retains full authority over it.
    /// Deposit - Yes
    /// Withdraw - Yes  
    /// Revoke - No, Realm authority cannot revoke liquid tokens
    Liquid,

    /// Membership token is a token controlled by Realm authority
    /// Deposit - Yes, membership tokens can be deposited to gain governance
    /// power.
    /// The membership tokens are conventionally minted into the holding
    /// account to keep them out of members possession.
    /// Withdraw - No, after membership tokens are deposited they are no longer
    /// transferable and can't be withdrawn.
    /// Revoke - Yes, Realm authority can Revoke (burn) membership tokens.
    Membership,

    /// Dormant token is a token which is only a placeholder and its deposits
    /// are not accepted and not used for governance power within the Realm
    ///
    /// The Dormant token type is used when only a single voting population is
    /// operational. For example a Multisig starter DAO uses Council only
    /// and sets Community as Dormant to indicate its not utilized for any
    /// governance power. Once the starter DAO decides to decentralise then
    /// it can change the Community token to Liquid
    ///
    /// Note: When an external voter weight plugin which takes deposits of the
    /// token is used then the type should be set to Dormant to make the
    /// intention explicit
    ///
    /// Deposit - No, dormant tokens can't be deposited into the Realm
    /// Withdraw - Yes, tokens can still be withdrawn from Realm to support
    /// scenario where the config is changed while some tokens are still
    /// deposited.
    /// Revoke - No, Realm authority cannot revoke dormant tokens
    Dormant,
}

impl Default for GoverningTokenType {
    fn default() -> Self {
        GoverningTokenType::Liquid
    }
}

/// The source of max vote weight used for voting
/// Values below 100% mint supply can be used when the governing token is fully
/// minted but not distributed yet
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub enum MintMaxVoterWeightSource {
    /// Fraction (10^10 precision) of the governing mint supply is used as max
    /// vote weight The default is 100% (10^10) to use all available mint
    /// supply for voting
    SupplyFraction(u64),

    /// Absolute value, irrelevant of the actual mint supply, is used as max
    /// voter weight
    Absolute(u64),
}

// /// Governance config
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// The type of the vote threshold used for community vote
    /// Note: In the current version only YesVotePercentage and Disabled
    /// thresholds are supported
    pub community_vote_threshold: VoteThreshold,

    /// Minimum community weight a governance token owner must possess to be
    /// able to create a proposal
    pub min_community_weight_to_create_proposal: u64,

    /// The wait time in seconds before transactions can be executed after
    /// proposal is successfully voted on
    pub transactions_hold_up_time: u32,

    /// The base voting time in seconds for proposal to be open for voting
    /// Voting is unrestricted during the base voting time and any vote types
    /// can be cast The base voting time can be extend by optional cool off
    /// time when only negative votes (Veto and Deny) are allowed
    pub voting_base_time: u32,

    /// Conditions under which a Community vote will complete early
    pub community_vote_tipping: VoteTipping,

    /// The type of the vote threshold used for council vote
    /// Note: In the current version only YesVotePercentage and Disabled
    /// thresholds are supported
    pub council_vote_threshold: VoteThreshold,

    /// The threshold for Council Veto votes
    pub council_veto_vote_threshold: VoteThreshold,

    /// Minimum council weight a governance token owner must possess to be able
    /// to create a proposal
    pub min_council_weight_to_create_proposal: u64,

    /// Conditions under which a Council vote will complete early
    pub council_vote_tipping: VoteTipping,

    /// The threshold for Community Veto votes
    pub community_veto_vote_threshold: VoteThreshold,

    /// Voting cool of time
    pub voting_cool_off_time: u32,

    /// The number of active proposals exempt from the Proposal security deposit
    pub deposit_exempt_proposal_count: u8,
}

/// The type of vote tipping to use on a Proposal.
///
/// Vote tipping means that under some conditions voting will complete early.
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum VoteTipping {
    /// Tip when there is no way for another option to win and the vote
    /// threshold has been reached. This ignores voters withdrawing their
    /// votes.
    ///
    /// Currently only supported for the "yes" option in single choice votes.
    Strict,

    /// Tip when an option reaches the vote threshold and has more vote weight
    /// than any other options.
    ///
    /// Currently only supported for the "yes" option in single choice votes.
    Early,

    /// Never tip the vote early.
    Disabled,
}

/// The type of the vote threshold used to resolve a vote on a Proposal
///
/// Note: In the current version only YesVotePercentage and Disabled thresholds
/// are supported
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum VoteThreshold {
    /// Voting threshold of Yes votes in % required to tip the vote (Approval
    /// Quorum) It's the percentage of tokens out of the entire pool of
    /// governance tokens eligible to vote Note: If the threshold is below
    /// or equal to 50% then an even split of votes ex: 50:50 or 40:40 is always
    /// resolved as Defeated In other words a '+1 vote' tie breaker is
    /// always required to have a successful vote
    YesVotePercentage(u8),

    /// The minimum number of votes in % out of the entire pool of governance
    /// tokens eligible to vote which must be cast for the vote to be valid
    /// Once the quorum is achieved a simple majority (50%+1) of Yes votes is
    /// required for the vote to succeed Note: Quorum is not implemented in
    /// the current version
    QuorumPercentage(u8),

    /// Disabled vote threshold indicates the given voting population (community
    /// or council) is not allowed to vote on proposals for the given
    /// Governance
    Disabled,
    //
    // Absolute vote threshold expressed in the voting mint units
    // It can be implemented once Solana runtime supports accounts resizing to accommodate u64
    // size extension Alternatively we could use the reserved space if it becomes a priority
    // Absolute(u64)
    //
    // Vote threshold which is always accepted
    // It can be used in a setup where the only security gate is proposal creation
    // and once created it's automatically approved
    // Any
}

/// Adds realm config account and accounts referenced by the config
/// 1) VoterWeightRecord
/// 2) MaxVoterWeightRecord
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
        true
    } else {
        false
    };

    if let Some(max_voter_weight_record) = max_voter_weight_record {
        accounts.push(AccountMeta::new_readonly(max_voter_weight_record, false));
        true
    } else {
        false
    };
}

/// Proposal vote type
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum VoteType {
    /// Single choice vote with mutually exclusive choices
    /// In the SingeChoice mode there can ever be a single winner
    /// If multiple options score the same highest vote then the Proposal is
    /// not resolved and considered as Failed.
    /// Note: Yes/No vote is a single choice (Yes) vote with the deny
    /// option (No)
    SingleChoice,

    /// Multiple options can be selected with up to max_voter_options per voter
    /// and with up to max_winning_options of successful options
    /// Ex. voters are given 5 options, can choose up to 3 (max_voter_options)
    /// and only 1 (max_winning_options) option can win and be executed
    MultiChoice {
        /// Type of MultiChoice
        #[allow(dead_code)]
        choice_type: MultiChoiceType,

        /// The min number of options a voter must choose
        ///
        /// Note: In the current version the limit is not supported and not
        /// enforced and must always be set to 1
        #[allow(dead_code)]
        min_voter_options: u8,

        /// The max number of options a voter can choose
        ///
        /// Note: In the current version the limit is not supported and not
        /// enforced and must always be set to the number of available
        /// options
        #[allow(dead_code)]
        max_voter_options: u8,

        /// The max number of wining options
        /// For executable proposals it limits how many options can be executed
        /// for a Proposal
        ///
        /// Note: In the current version the limit is not supported and not
        /// enforced and must always be set to the number of available
        /// options
        #[allow(dead_code)]
        max_winning_options: u8,
    },
}

/// Type of MultiChoice.
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum MultiChoiceType {
    /// Multiple options can be approved with full weight allocated to each
    /// approved option
    FullWeight,

    /// Multiple options can be approved with weight allocated proportionally
    /// to the percentage of the total weight.
    /// The full weight has to be voted among the approved options, i.e.,
    /// 100% of the weight has to be allocated
    Weighted,
}

#[derive(Debug, Copy, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
/// Enum to specify the authority by which the instruction should add a
/// signatory
pub enum AddSignatoryAuthority {
    /// Proposal owners can add optional signatories to a proposal
    ProposalOwner {
        /// Token owner or its delegate
        governance_authority: Pubkey,
        /// Token owner record of the Proposal owner
        token_owner_record: Pubkey,
    },
    /// Anyone can add signatories that are required by the governance to a
    /// proposal
    None,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
/// Enum to specify the authority by which the instruction should add a
/// signatory
pub enum AddSignatoryAuthoritySPO {
    /// Proposal owners can add optional signatories to a proposal
    ProposalOwner {
        /// Token owner or its delegate
        governance_authority: String,
        /// Token owner record of the Proposal owner
        token_owner_record: String,
    },
    /// Anyone can add signatories that are required by the governance to a
    /// proposal
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
    /// Vote approving choices
    Approve(Vec<VoteChoice>),

    /// Vote rejecting proposal
    Deny,

    /// Declare indifference to proposal
    /// Note: Not supported in the current version
    Abstain,

    /// Veto proposal
    Veto,
}

/// Voter choice for a proposal option
/// In the current version only 1) Single choice, 2) Multiple choices proposals
/// and 3) Weighted voting are supported.
/// In the future versions we can add support for 1) Quadratic voting and
/// 2) Ranked choice voting
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct VoteChoice {
    /// The rank given to the choice by voter
    /// Note: The field is not used in the current version
    pub rank: u8,

    /// The voter's weight percentage given by the voter to the choice
    pub weight_percentage: u8,
}

/// InstructionData wrapper. It can be removed once Borsh serialization for
/// Instruction is supported in the SDK
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct InstructionData {
    /// Pubkey of the instruction processor that executes this instruction
    pub program_id: Pubkey,
    /// Metadata for what accounts should be passed to the instruction processor
    pub accounts: Vec<AccountMetaData>,
    /// Opaque data passed to the instruction processor
    pub data: Vec<u8>,
}

/// Account metadata used to define Instructions
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct AccountMetaData {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if an Instruction requires a Transaction signature matching
    /// `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
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

/// SetRealmConfigItem instruction arguments to set a single Realm config item
/// Note: In the current version only TokenOwnerRecordLockAuthority is supported
/// Eventually all Realm config items should be supported for single config item
/// change
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum SetRealmConfigItemArgs {
    /// Set TokenOwnerRecord lock authority
    TokenOwnerRecordLockAuthority {
        /// Action indicating whether to add or remove the lock authority
        #[allow(dead_code)]
        action: SetConfigItemActionType,
        /// Mint of the governing token the lock authority is for
        #[allow(dead_code)]
        governing_token_mint: Pubkey,
        /// Authority to change
        #[allow(dead_code)]
        authority: Pubkey,
    },
}

/// Enum describing the action type for setting a config item
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum SetConfigItemActionType {
    /// Add config item
    Add,

    /// Remove config item
    Remove,
}

/// SetRealmAuthority instruction action
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum SetRealmAuthorityAction {
    /// Sets realm authority without any checks
    /// Uncheck option allows to set the realm authority to non governance
    /// accounts
    SetUnchecked,

    /// Sets realm authority and checks the new new authority is one of the
    /// realm's governances
    // Note: This is not a security feature because governance creation is only
    // gated with min_community_weight_to_create_governance.
    // The check is done to prevent scenarios where the authority could be
    // accidentally set to a wrong or none existing account.
    SetChecked,

    /// Removes realm authority
    Remove,
}
