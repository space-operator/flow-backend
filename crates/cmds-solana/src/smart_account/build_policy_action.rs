use super::pda;
use crate::prelude::*;
use tracing::info;

const NAME: &str = "smart_account_build_policy_action";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/build_policy_action.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsPubkey")]
    pub settings: Pubkey,
    pub policy_seed: u64,
    pub policy_type: String,
    // SpendingLimit fields
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub mint: Option<Pubkey>,
    #[serde(default)]
    pub account_index: Option<u8>,
    #[serde(default)]
    pub amount: Option<u64>,
    #[serde(default)]
    pub max_per_use: Option<u64>,
    #[serde(default)]
    pub period: Option<u8>,
    #[serde_as(as = "Option<Vec<AsPubkey>>")]
    #[serde(default)]
    pub destinations: Option<Vec<Pubkey>>,
    // SettingsChange fields
    #[serde(default)]
    pub allowed_actions: Option<Vec<String>>,
    // ProgramInteraction fields
    /// Target program ID for instruction constraints
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub target_program: Option<Pubkey>,
    /// Instruction data constraint: offset to check
    #[serde(default)]
    pub data_offset: Option<u32>,
    /// Instruction data constraint: expected value bytes
    #[serde(default)]
    pub data_value: Option<Vec<u8>>,
    /// Instruction data constraint: operator (0=Eq, 1=Neq, 2=Gt, 3=Lt, 4=Gte, 5=Lte)
    #[serde(default)]
    pub data_operator: Option<u8>,
    /// Pre-hook program ID
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub pre_hook_program: Option<Pubkey>,
    /// Post-hook program ID
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub post_hook_program: Option<Pubkey>,
    // Common policy fields
    #[serde_as(as = "Vec<AsPubkey>")]
    pub policy_signers: Vec<Pubkey>,
    #[serde(default = "default_permissions")]
    pub permissions_mask: u8,
    #[serde(default = "default_threshold")]
    pub threshold: u16,
    #[serde(default)]
    pub time_lock: u32,
}

fn default_permissions() -> u8 {
    7
}
fn default_threshold() -> u16 {
    1
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub actions: Vec<u8>,
    /// Policy PDA as a single-element array for wiring to execute_settings_transaction.policy_pdas
    #[serde_as(as = "Vec<AsPubkey>")]
    pub policy_pdas: Vec<Pubkey>,
    pub transaction_index: u64,
}

/// Read the settings account's transaction_index from on-chain data.
/// Settings layout (after 8-byte discriminator):
///   createKey: Pubkey (32)       — offset 8
///   accountIndex: u128 (16)      — offset 40
///   threshold: u16 (2)           — offset 56
///   timeLock: u32 (4)            — offset 58
///   transactionIndex: u64 (8)    — offset 62
///   staleTransactionIndex: u64 (8) — offset 70
///   ...
fn read_transaction_index(data: &[u8]) -> Result<u64, CommandError> {
    const TX_INDEX_OFFSET: usize = 62;
    if data.len() < TX_INDEX_OFFSET + 8 {
        return Err(CommandError::msg(
            "Settings account data too short for transaction_index",
        ));
    }
    let tx_index = u64::from_le_bytes(
        data[TX_INDEX_OFFSET..TX_INDEX_OFFSET + 8]
            .try_into()
            .map_err(|_| CommandError::msg("Invalid transaction_index bytes"))?,
    );
    Ok(tx_index)
}

/// Read the settings account's policy_seed from on-chain data.
/// The policy_seed is stored as Option<u64> near the end of the settings data.
/// Offset 158: Option<u64> — 0=None, 1=Some(seed)
fn read_policy_seed(data: &[u8]) -> Result<Option<u64>, CommandError> {
    const POLICY_SEED_OFFSET: usize = 158;
    if data.len() < POLICY_SEED_OFFSET + 9 {
        return Ok(None); // Account too short, no policy_seed field
    }
    if data[POLICY_SEED_OFFSET] == 0 {
        return Ok(None);
    }
    let seed = u64::from_le_bytes(
        data[POLICY_SEED_OFFSET + 1..POLICY_SEED_OFFSET + 9]
            .try_into()
            .map_err(|_| CommandError::msg("Invalid policy_seed bytes"))?,
    );
    Ok(Some(seed))
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // 1. Read settings account to get next transaction_index AND policy_seed
    let (transaction_index, next_policy_seed) = {
        let client = ctx.solana_client();
        let settings_data = client
            .get_account_data(&input.settings)
            .await
            .map_err(|e| CommandError::msg(format!("Failed to read settings account: {e}")))?;
        let current_tx = read_transaction_index(&settings_data)?;
        let current_policy = read_policy_seed(&settings_data)?;
        // The program uses transaction_index + 1 and policy_seed + 1
        let next_policy = current_policy.map(|s| s + 1).unwrap_or(input.policy_seed);
        (current_tx + 1, next_policy)
    };

    // Use the auto-detected policy seed, or fall back to user-provided
    let effective_seed = next_policy_seed;

    // 2. Derive policy PDA using the effective seed
    let (policy_pda, _) = pda::find_policy(&input.settings, effective_seed);

    info!(
        "build_policy_action: policy_type={}, seed={} (user={}, auto={}), policy_pda={}, transaction_index={}",
        input.policy_type,
        effective_seed,
        input.policy_seed,
        next_policy_seed,
        policy_pda,
        transaction_index
    );

    // 3. Serialize SettingsAction::PolicyCreate
    let mut actions = Vec::new();

    // Vec<SettingsAction> length = 1
    actions.extend_from_slice(&1u32.to_le_bytes());

    // SettingsAction variant 7 = PolicyCreate (Borsh enum = u8)
    actions.push(7);

    // seed: u64 — the program auto-increments, but we still pass a value
    actions.extend_from_slice(&effective_seed.to_le_bytes());

    // PolicyCreationPayload (u8 discriminant)
    match input.policy_type.as_str() {
        "spending_limit" => {
            actions.push(1); // SpendingLimit

            // mint: Pubkey (default = native SOL)
            let mint = input.mint.unwrap_or_default();
            actions.extend_from_slice(mint.as_ref());

            // sourceAccountIndex: u8
            actions.push(input.account_index.unwrap_or(0));

            // timeConstraints:
            //   start: i64 = 0 (immediate)
            actions.extend_from_slice(&0i64.to_le_bytes());
            //   expiration: Option<i64> = None
            actions.push(0);
            //   period: PeriodV2 (u8)
            actions.push(input.period.unwrap_or(0)); // 0=OneTime
            //   accumulateUnused: bool
            actions.push(0);

            // quantityConstraints:
            let amount = input.amount.unwrap_or(1_000_000);
            let max_per_use = input.max_per_use.unwrap_or(amount);
            //   maxPerPeriod: u64
            actions.extend_from_slice(&amount.to_le_bytes());
            //   maxPerUse: u64
            actions.extend_from_slice(&max_per_use.to_le_bytes());
            //   enforceExactQuantity: bool
            actions.push(0);

            // usageState: Option = None
            actions.push(0);

            // destinations: Vec<Pubkey>
            let dests = input.destinations.as_deref().unwrap_or(&[]);
            actions.extend_from_slice(&(dests.len() as u32).to_le_bytes());
            for d in dests {
                actions.extend_from_slice(d.as_ref());
            }
        }
        "settings_change" => {
            actions.push(2); // SettingsChange

            // AllowedSettingsChange Vec
            let allowed = input.allowed_actions.as_deref().unwrap_or(&[]);
            actions.extend_from_slice(&(allowed.len() as u32).to_le_bytes());
            for act in allowed {
                match act.as_str() {
                    "add_signer" => {
                        actions.push(0); // AddSigner
                        actions.push(0); // key: Option<Pubkey> = None
                        actions.push(0); // permissions: Option = None
                    }
                    "remove_signer" => {
                        actions.push(1); // RemoveSigner
                        actions.push(0); // key: Option<Pubkey> = None
                    }
                    "change_threshold" => {
                        actions.push(2); // ChangeThreshold
                    }
                    "change_time_lock" => {
                        actions.push(3); // ChangeTimeLock
                        actions.push(0); // value: Option<u32> = None
                    }
                    other => {
                        return Err(CommandError::msg(format!(
                            "Unknown settings change action: {other}"
                        )));
                    }
                }
            }
        }
        "internal_fund_transfer" => {
            actions.push(0); // InternalFundTransfer

            // sourceAccountIndices: Vec<u8> — list of allowed source account indices
            // Default: [0] (account 0 only)
            actions.extend_from_slice(&1u32.to_le_bytes()); // Vec length = 1
            actions.push(0); // account index 0

            // destinationAccountIndices: Vec<u8> — list of allowed destination account indices
            // Default: [1] (account 1 only)
            actions.extend_from_slice(&1u32.to_le_bytes()); // Vec length = 1
            actions.push(1); // account index 1

            // allowedMints: Vec<Pubkey>
            // Default: [Pubkey::default] (native SOL)
            actions.extend_from_slice(&1u32.to_le_bytes()); // Vec length = 1
            actions.extend_from_slice(Pubkey::default().as_ref()); // native SOL
        }
        "program_interaction" => {
            actions.push(3); // ProgramInteraction

            // Borsh field order: accountIndex, instructionsConstraints, preHook, postHook, spendingLimits

            // 1. accountIndex: u8
            actions.push(input.account_index.unwrap_or(0));

            // 2. instructionsConstraints: Vec<InstructionConstraint>
            //    Field order: programId, accountConstraints, dataConstraints
            if let Some(ref target) = input.target_program {
                actions.extend_from_slice(&1u32.to_le_bytes()); // 1 constraint
                actions.extend_from_slice(target.as_ref()); // programId
                actions.extend_from_slice(&0u32.to_le_bytes()); // accountConstraints: empty
                // dataConstraints: Vec<DataConstraint>
                if let (Some(offset), Some(value), Some(op)) = (
                    input.data_offset,
                    input.data_value.as_ref(),
                    input.data_operator,
                ) {
                    actions.extend_from_slice(&1u32.to_le_bytes()); // 1 constraint
                    actions.extend_from_slice(&(offset as u64).to_le_bytes()); // dataOffset: u64
                    // dataValue: 0=U8, 1=U16Le, 2=U32Le, 3=U64Le, 4=U128Le, 5=U8Slice
                    if value.len() == 1 {
                        actions.push(0); // U8
                        actions.push(value[0]);
                    } else {
                        actions.push(5); // U8Slice
                        actions.extend_from_slice(&(value.len() as u32).to_le_bytes());
                        actions.extend_from_slice(value);
                    }
                    actions.push(op); // DataOperator: 0=Eq, 1=Neq, 2=Gt, 3=Gte, 4=Lt, 5=Lte
                } else {
                    actions.extend_from_slice(&0u32.to_le_bytes()); // 0 data constraints
                }
            } else {
                actions.extend_from_slice(&0u32.to_le_bytes()); // 0 constraints
            }

            // 3. preHook: Option<Hook>
            //    Hook field order: numExtraAccounts, accountConstraints, instructionData, programId, passInnerInstructions
            if let Some(ref hook_program) = input.pre_hook_program {
                actions.push(1); // Some
                actions.push(0); // numExtraAccounts
                actions.extend_from_slice(&0u32.to_le_bytes()); // accountConstraints: empty
                actions.extend_from_slice(&0u32.to_le_bytes()); // instructionData: empty
                actions.extend_from_slice(hook_program.as_ref()); // programId
                actions.push(0); // passInnerInstructions
            } else {
                actions.push(0); // None
            }

            // 4. postHook: Option<Hook>
            if let Some(ref hook_program) = input.post_hook_program {
                actions.push(1); // Some
                actions.push(0); // numExtraAccounts
                actions.extend_from_slice(&0u32.to_le_bytes()); // accountConstraints
                actions.extend_from_slice(&0u32.to_le_bytes()); // instructionData
                actions.extend_from_slice(hook_program.as_ref()); // programId
                actions.push(0); // passInnerInstructions
            } else {
                actions.push(0); // None
            }

            // 5. spendingLimits: Vec<LimitedSpendingLimit> = empty
            actions.extend_from_slice(&0u32.to_le_bytes());
        }
        other => {
            return Err(CommandError::msg(format!(
                "Unsupported policy type: {other}. Use: spending_limit, settings_change, internal_fund_transfer, program_interaction"
            )));
        }
    }

    // Common policy fields:

    // signers: Vec<SmartAccountSigner>
    actions.extend_from_slice(&(input.policy_signers.len() as u32).to_le_bytes());
    for signer in &input.policy_signers {
        actions.extend_from_slice(signer.as_ref()); // key: Pubkey
        actions.push(input.permissions_mask); // permissions.mask: u8
    }

    // threshold: u16
    actions.extend_from_slice(&input.threshold.to_le_bytes());

    // timeLock: u32
    actions.extend_from_slice(&input.time_lock.to_le_bytes());

    // startTimestamp: Option<i64> = None
    actions.push(0);

    // expirationArgs: Option = None
    actions.push(0);

    info!(
        "build_policy_action: serialized {} bytes for policy_type={}",
        actions.len(),
        input.policy_type
    );

    Ok(Output {
        actions,
        policy_pdas: vec![policy_pda],
        transaction_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
