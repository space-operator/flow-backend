use super::pda;
use crate::prelude::*;
use tracing::info;

const NAME: &str = "smart_account_build_policy_action";
const DEFINITION: &str =
    flow_lib::node_definition!("smart_account/build_policy_action.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?.check_name(NAME)
    });
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
        return Err(CommandError::msg("Settings account data too short for transaction_index"));
    }
    let tx_index = u64::from_le_bytes(
        data[TX_INDEX_OFFSET..TX_INDEX_OFFSET + 8]
            .try_into()
            .map_err(|_| CommandError::msg("Invalid transaction_index bytes"))?,
    );
    Ok(tx_index)
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // 1. Derive policy PDA
    let (policy_pda, _) = pda::find_policy(&input.settings, input.policy_seed);

    // 2. Read settings account to get next transaction_index
    let transaction_index = {
        let client = ctx.solana_client();
        let settings_data = client
            .get_account_data(&input.settings)
            .await
            .map_err(|e| CommandError::msg(format!("Failed to read settings account: {e}")))?;
        let current = read_transaction_index(&settings_data)?;
        current + 1 // The program uses transaction_index + 1
    };

    info!(
        "build_policy_action: policy_type={}, seed={}, policy_pda={}, transaction_index={}",
        input.policy_type, input.policy_seed, policy_pda, transaction_index
    );

    // 3. Serialize SettingsAction::PolicyCreate
    let mut actions = Vec::new();

    // Vec<SettingsAction> length = 1
    actions.extend_from_slice(&1u32.to_le_bytes());

    // SettingsAction variant 7 = PolicyCreate (Borsh enum = u8)
    actions.push(7);

    // seed: u64
    actions.extend_from_slice(&input.policy_seed.to_le_bytes());

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

            // source_account_mask: [u8; 32] — bit 0 = account 0, etc.
            // Default: only account 0
            let mut source_mask = [0u8; 32];
            source_mask[0] = 1; // bit 0
            actions.extend_from_slice(&source_mask);

            // destination_account_mask: [u8; 32]
            let mut dest_mask = [0u8; 32];
            dest_mask[0] = 2; // bit 1 (account 1)
            actions.extend_from_slice(&dest_mask);

            // allowed_mints: Vec<Pubkey> = empty (any)
            actions.extend_from_slice(&0u32.to_le_bytes());
        }
        other => {
            return Err(CommandError::msg(format!(
                "Unsupported policy type: {other}. Use: spending_limit, settings_change, internal_fund_transfer"
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
