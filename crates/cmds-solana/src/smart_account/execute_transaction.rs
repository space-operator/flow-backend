use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_program::instruction::AccountMeta;
use tracing::info;

const NAME: &str = "smart_account_execute_transaction";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/execute_transaction.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub settings: Pubkey,
    pub signer: Wallet,
    pub transaction_index: u64,
    #[serde(default)]
    pub account_index: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// Parsed data from a VaultTransaction account needed to build remaining_accounts.
struct ParsedTransactionMessage {
    /// Ephemeral signer bump seeds from the on-chain data.
    ephemeral_signer_bumps: Vec<u8>,
    /// Static account keys from the message with correct writable flags.
    /// All accounts have is_signer=false — the program handles signing via CPI.
    account_metas: Vec<AccountMeta>,
    /// Address lookup table keys referenced by the message.
    /// These ALT account pubkeys must come FIRST in remaining_accounts.
    address_lookup_table_keys: Vec<Pubkey>,
    /// Accounts resolved from address lookup tables: writable first, then readonly.
    /// These come AFTER static account keys in remaining_accounts.
    lookup_resolved_metas: Vec<AccountMeta>,
}

/// Parse the SmartAccountTransactionMessage stored in a vault transaction account
/// to extract all data needed for building remaining_accounts.
///
/// On-chain VaultTransaction layout (after 8-byte Anchor discriminator):
///   settings: Pubkey (32)
///   creator: Pubkey (32)
///   rentCollector: Pubkey (32)
///   index: u64 (8)
///   bump: u8 (1)
///   account_index: u8 (1)
///   ephemeral_signer_bumps: Vec<u8> (4 + N)
///   message: SmartAccountTransactionMessage
///
/// SmartAccountTransactionMessage (Borsh):
///   num_signers: u8
///   num_writable_signers: u8
///   num_writable_non_signers: u8
///   account_keys: Vec<Pubkey> (4 + N*32)
///   instructions: Vec<SmartAccountCompiledInstruction>
///   address_table_lookups: Vec<SmartAccountMessageAddressTableLookup>
fn parse_vault_transaction(data: &[u8]) -> Result<ParsedTransactionMessage, CommandError> {
    // VaultTransaction fixed header:
    //   disc: 8 + settings: 32 + creator: 32 + rentCollector: 32 + index: 8 + bump: 1 + accountIndex: 1 = 114
    let mut offset = 114;

    // ephemeral_signer_bumps: Vec<u8>
    if offset + 4 > data.len() {
        return Err(CommandError::msg(
            "Transaction data too short for ephemeral_signer_bumps",
        ));
    }
    let bumps_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let ephemeral_signer_bumps = data[offset..offset + bumps_len].to_vec();
    offset += bumps_len;

    // SmartAccountTransactionMessage — stored on-chain in Borsh format.
    // The create_transaction instruction receives compact-encoded bytes but the
    // program deserializes and re-serializes as Borsh in the VaultTransaction account.
    //
    // Borsh layout:
    //   num_signers: u8
    //   num_writable_signers: u8
    //   num_writable_non_signers: u8
    //   account_keys: Vec<Pubkey> (u32 LE length + N*32)
    //   instructions: Vec<CompiledInstruction> (u32 LE length + ...)
    //   address_table_lookups: Vec<AddressTableLookup> (u32 LE length + ...)

    if offset + 3 > data.len() {
        return Err(CommandError::msg(
            "Transaction data too short for message header",
        ));
    }
    let _num_signers = data[offset] as usize;
    let num_writable_signers = data[offset + 1] as usize;
    let num_writable_non_signers = data[offset + 2] as usize;
    offset += 3;

    // account_keys: Vec<Pubkey> (Borsh: u32 LE length + keys)
    if offset + 4 > data.len() {
        return Err(CommandError::msg(
            "Transaction data too short for account_keys length",
        ));
    }
    let num_keys = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    let mut account_metas = Vec::with_capacity(num_keys);
    for i in 0..num_keys {
        if offset + 32 > data.len() {
            return Err(CommandError::msg(
                "Transaction data too short for account key",
            ));
        }
        let key = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| CommandError::msg("Invalid account key"))?;
        offset += 32;

        // Writable flags from the message header layout:
        // [0..num_writable_signers) = writable signers
        // [num_writable_signers..num_signers) = readonly signers
        // [num_signers..num_signers+num_writable_non_signers) = writable non-signers
        // [num_signers+num_writable_non_signers..) = readonly non-signers
        let is_writable = i < num_writable_signers
            || (i >= _num_signers && i < _num_signers + num_writable_non_signers);

        // Never mark remaining_accounts as signers. The Squads program handles
        // all inner-instruction signing via invoke_signed (vault PDA + ephemeral
        // signers). Setting is_signer=true would cause VersionedTransaction::try_new
        // to require keypairs we don't have.
        account_metas.push(AccountMeta {
            pubkey: key,
            is_signer: false,
            is_writable,
        });
    }

    // instructions: Vec<CompiledInstruction> (Borsh) — skip to reach address_table_lookups
    if offset + 4 > data.len() {
        return Err(CommandError::msg(
            "Transaction data too short for instructions length",
        ));
    }
    let num_instructions =
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    for _ in 0..num_instructions {
        if offset >= data.len() {
            return Err(CommandError::msg(
                "Transaction data too short for instruction",
            ));
        }
        offset += 1; // program_id_index: u8

        // account_indexes: Vec<u8> (Borsh: u32 + N)
        if offset + 4 > data.len() {
            return Err(CommandError::msg(
                "Transaction data too short for account_indexes",
            ));
        }
        let ai_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4 + ai_len;

        // data: Vec<u8> (Borsh: u32 + N)
        if offset + 4 > data.len() {
            return Err(CommandError::msg(
                "Transaction data too short for instruction data",
            ));
        }
        let ix_data_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4 + ix_data_len;
    }

    // address_table_lookups: Vec<AddressTableLookup> (Borsh)
    let mut address_lookup_table_keys = Vec::new();
    let lookup_resolved_metas = Vec::new();

    if offset + 4 <= data.len() {
        let num_lookups = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        for _ in 0..num_lookups {
            if offset + 32 > data.len() {
                return Err(CommandError::msg(
                    "Transaction data too short for lookup table key",
                ));
            }
            let table_key = Pubkey::try_from(&data[offset..offset + 32])
                .map_err(|_| CommandError::msg("Invalid lookup table key"))?;
            offset += 32;
            address_lookup_table_keys.push(table_key);

            // writable_indexes: Vec<u8> (Borsh: u32 + N)
            if offset + 4 > data.len() {
                return Err(CommandError::msg(
                    "Transaction data too short for writable_indexes",
                ));
            }
            let writable_len =
                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4 + writable_len;

            // readonly_indexes: Vec<u8> (Borsh: u32 + N)
            if offset + 4 > data.len() {
                return Err(CommandError::msg(
                    "Transaction data too short for readonly_indexes",
                ));
            }
            let readonly_len =
                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4 + readonly_len;
        }
    }

    Ok(ParsedTransactionMessage {
        ephemeral_signer_bumps,
        account_metas,
        address_lookup_table_keys,
        lookup_resolved_metas,
    })
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (proposal, _) = pda::find_proposal(&input.settings, input.transaction_index);
    let (transaction, _) = pda::find_transaction(&input.settings, input.transaction_index);

    // Derive the vault (smart account) PDA
    let (vault, _) = pda::find_smart_account(&input.settings, input.account_index);

    // Read the vault transaction account to get the inner instruction accounts.
    // Retry a few times since the account may have been created in the same flow
    // run and not yet visible at the current commitment level.
    let tx_data = {
        let client = ctx.solana_client();
        let mut data = None;
        for attempt in 0..10u32 {
            match client.get_account_data(&transaction).await {
                Ok(d) => {
                    data = Some(d);
                    break;
                }
                Err(_) if attempt < 9 => {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                }
                Err(e) => {
                    return Err(CommandError::msg(format!(
                        "Failed to read transaction account after retries: {e}"
                    )));
                }
            }
        }
        data.ok_or_else(|| CommandError::msg("Transaction account not found after retries"))?
    };

    // Parse the stored message to get all data for remaining_accounts.
    let parsed = parse_vault_transaction(&tx_data)?;

    info!(
        "execute_transaction: vault={}, transaction={}, data_len={}, \
         ephemeral_bumps={:?}, {} account_keys, {} ALT keys",
        vault,
        transaction,
        tx_data.len(),
        parsed.ephemeral_signer_bumps,
        parsed.account_metas.len(),
        parsed.address_lookup_table_keys.len(),
    );

    // Build remaining_accounts in the order the Squads program expects:
    //   1. Address lookup table accounts (the table account pubkeys themselves)
    //   2. Static account keys from the message (with correct writable/signer flags)
    //   3. Accounts resolved from lookup tables (writable first, then readonly)
    //      Note: for now we don't resolve ALT entries — the program does this on-chain
    let mut remaining_accounts: Vec<AccountMeta> = Vec::new();

    // 1. ALT accounts (always readonly, non-signer)
    for alt_key in &parsed.address_lookup_table_keys {
        remaining_accounts.push(AccountMeta::new_readonly(*alt_key, false));
    }

    // 2. Static account keys
    remaining_accounts.extend(parsed.account_metas.iter().cloned());

    // 3. Lookup-resolved accounts (currently empty for simple transactions)
    remaining_accounts.extend(parsed.lookup_resolved_metas.iter().cloned());

    // Build the accounts list: 5 named accounts + remaining_accounts
    // The Squads execute_transaction Anchor struct:
    //   0: settings (writable)
    //   1: proposal (writable)
    //   2: transaction (readonly)
    //   3: signer (signer)
    //   4: program (readonly) — Anchor validates this equals the program ID
    // Everything after position 4 is remaining_accounts.
    let mut accounts = vec![
        AccountMeta::new(input.settings, false),
        AccountMeta::new(proposal, false),
        AccountMeta::new_readonly(transaction, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new_readonly(super::PROGRAM_ID, false),
    ];
    accounts.extend(remaining_accounts);

    let instruction = build_instruction("execute_transaction", accounts, vec![]);

    info!(
        "execute_transaction instruction: program={}, {} accounts, {} bytes data",
        instruction.program_id,
        instruction.accounts.len(),
        instruction.data.len()
    );
    for (idx, acc) in instruction.accounts.iter().enumerate() {
        info!(
            "  account[{}]: {} (signer={}, writable={})",
            idx, acc.pubkey, acc.is_signer, acc.is_writable
        );
    }

    // The Squads smart-account program requires more than the default 32KB heap.
    let heap_ix = ComputeBudgetInstruction::request_heap_frame(256 * 1024);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.signer.clone()]
            .into_iter()
            .collect(),
        instructions: vec![heap_ix, instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Build a minimal VaultTransaction account data for testing the parser.
    fn build_test_vault_tx_data(
        settings: &Pubkey,
        creator: &Pubkey,
        ephemeral_bumps: &[u8],
        account_keys: &[Pubkey],
        num_signers: u8,
        num_writable_signers: u8,
        num_writable_non_signers: u8,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        // 8-byte discriminator
        data.extend_from_slice(&[0u8; 8]);
        // settings: Pubkey
        data.extend_from_slice(settings.as_ref());
        // creator: Pubkey
        data.extend_from_slice(creator.as_ref());
        // rentCollector: Pubkey
        data.extend_from_slice(Pubkey::default().as_ref());
        // index: u64
        data.extend_from_slice(&1u64.to_le_bytes());
        // bump: u8
        data.push(255);
        // accountIndex: u8
        data.push(0);

        // ephemeral_signer_bumps: Vec<u8>
        data.extend_from_slice(&(ephemeral_bumps.len() as u32).to_le_bytes());
        data.extend_from_slice(ephemeral_bumps);

        // SmartAccountTransactionMessage (Borsh format, as stored on-chain):
        data.push(num_signers);
        data.push(num_writable_signers);
        data.push(num_writable_non_signers);

        // account_keys: Vec<Pubkey> (Borsh: u32 LE length + keys)
        data.extend_from_slice(&(account_keys.len() as u32).to_le_bytes());
        for key in account_keys {
            data.extend_from_slice(key.as_ref());
        }

        // instructions: Vec (Borsh: u32 = 0)
        data.extend_from_slice(&0u32.to_le_bytes());

        // address_table_lookups: Vec (Borsh: u32 = 0)
        data.extend_from_slice(&0u32.to_le_bytes());

        data
    }

    #[test]
    fn test_parse_simple_transaction() {
        let settings = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let system_program = solana_program::system_program::id();
        let transaction = Pubkey::new_unique();

        // 2 account keys: vault (writable signer), system_program (readonly non-signer)
        let data = build_test_vault_tx_data(
            &settings,
            &creator,
            &[],
            &[vault, system_program],
            1, // num_signers
            1, // num_writable_signers
            0, // num_writable_non_signers
        );

        let parsed = parse_vault_transaction(&data).unwrap();

        assert_eq!(parsed.ephemeral_signer_bumps.len(), 0);
        assert_eq!(parsed.account_metas.len(), 2);
        assert_eq!(parsed.address_lookup_table_keys.len(), 0);

        // All remaining_accounts have is_signer=false — program handles signing via CPI
        assert_eq!(parsed.account_metas[0].pubkey, vault);
        assert!(!parsed.account_metas[0].is_signer);
        assert!(parsed.account_metas[0].is_writable);

        assert_eq!(parsed.account_metas[1].pubkey, system_program);
        assert!(!parsed.account_metas[1].is_signer);
        assert!(!parsed.account_metas[1].is_writable);
    }

    #[test]
    fn test_parse_with_ephemeral_signers() {
        let settings = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let transaction = Pubkey::new_unique();

        let ephemeral_0 = pda::find_ephemeral_signer(&transaction, 0).0;

        // 2 account keys: vault (writable signer), ephemeral_0 (readonly signer)
        let data = build_test_vault_tx_data(
            &settings,
            &creator,
            &[254], // one ephemeral bump
            &[vault, ephemeral_0],
            2, // num_signers
            1, // num_writable_signers (only vault)
            0, // num_writable_non_signers
        );

        let parsed = parse_vault_transaction(&data).unwrap();

        assert_eq!(parsed.ephemeral_signer_bumps, vec![254]);
        assert_eq!(parsed.account_metas.len(), 2);

        // All remaining_accounts have is_signer=false
        assert!(!parsed.account_metas[0].is_signer);
        assert!(!parsed.account_metas[1].is_signer);
    }
}
