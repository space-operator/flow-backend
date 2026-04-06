use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "create_smart_account";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/create_smart_account.jsonc");

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
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub settings_authority: Option<Pubkey>,
    pub threshold: u16,
    pub signers: Vec<SmartAccountSignerInput>,
    #[serde(default)]
    pub time_lock: u32,
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub rent_collector: Option<Pubkey>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SmartAccountSignerInput {
    pub key: String,
    pub permissions: u8,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub program_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub settings: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (program_config, _) = pda::find_program_config();

    // Read program config on-chain to get treasury and smart_account_index.
    // Scope the client borrow so ctx can be mutably borrowed later by execute().
    let config_data = {
        let client = ctx.solana_client();
        client
            .get_account_data(&program_config)
            .await
            .map_err(|e| CommandError::msg(format!("Failed to read program config: {e}")))?
    };

    // ProgramConfig layout (after 8-byte Anchor discriminator):
    // smart_account_index: u128 (16 bytes)
    // authority: Pubkey (32 bytes)
    // smart_account_creation_fee: u64 (8 bytes)
    // treasury: Pubkey (32 bytes)
    if config_data.len() < 8 + 16 + 32 + 8 + 32 {
        return Err(CommandError::msg("Program config data too short"));
    }

    // Read smart_account_index (u128 LE at offset 8) for deriving the settings PDA
    let smart_account_index = u128::from_le_bytes(
        config_data[8..24]
            .try_into()
            .map_err(|_| CommandError::msg("Invalid smart_account_index"))?,
    );

    let treasury_offset = 8 + 16 + 32 + 8;
    let treasury = Pubkey::try_from(&config_data[treasury_offset..treasury_offset + 32])
        .map_err(|_| CommandError::msg("Invalid treasury pubkey"))?;

    // Derive settings PDAs for a range of indices. On a busy devnet the
    // smart_account_index may increment between our RPC read and tx
    // execution. The on-chain program scans remaining_accounts for the
    // PDA matching its atomic index, so we include speculative PDAs for
    // the next SPECULATION_RANGE indices to handle the race.
    const SPECULATION_RANGE: u128 = 10;
    let mut accounts = vec![
        // Named accounts (matching the Anchor Accounts struct)
        AccountMeta::new(program_config, false),
        AccountMeta::new(treasury, false),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
    ];

    // Add speculative settings PDAs as remaining accounts
    let mut settings = Pubkey::default();
    for offset in 0..SPECULATION_RANGE {
        let (pda, _) = pda::find_settings(smart_account_index + offset);
        if offset == 0 {
            settings = pda;
        }
        accounts.push(AccountMeta::new(pda, false));
    }

    // Serialize args: CreateSmartAccountArgs
    let mut args_data = Vec::new();

    tracing::info!(
        "create_smart_account: settings_authority = {:?}",
        input.settings_authority
    );

    // settings_authority: Option<Pubkey>
    match input.settings_authority {
        Some(pk) => {
            args_data.push(1);
            args_data.extend_from_slice(pk.as_ref());
        }
        None => args_data.push(0),
    }

    // threshold: u16
    args_data.extend_from_slice(&input.threshold.to_le_bytes());

    // signers: Vec<SmartAccountSigner>
    args_data.extend_from_slice(&(input.signers.len() as u32).to_le_bytes());
    for signer in &input.signers {
        let key: Pubkey = signer
            .key
            .parse()
            .map_err(|_| CommandError::msg(format!("Invalid signer key: {}", signer.key)))?;
        args_data.extend_from_slice(key.as_ref());
        // Permissions { mask: u8 }
        args_data.push(signer.permissions);
    }

    // time_lock: u32
    args_data.extend_from_slice(&input.time_lock.to_le_bytes());

    // rent_collector: Option<Pubkey>
    match input.rent_collector {
        Some(pk) => {
            args_data.push(1);
            args_data.extend_from_slice(pk.as_ref());
        }
        None => args_data.push(0),
    }

    // memo: Option<String>
    match &input.memo {
        Some(s) => {
            args_data.push(1);
            args_data.extend_from_slice(&(s.len() as u32).to_le_bytes());
            args_data.extend_from_slice(s.as_bytes());
        }
        None => args_data.push(0),
    }

    let instruction = build_instruction("create_smart_account", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone()].into_iter().collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    // After successful execution, find the actual settings PDA by scanning
    // the speculative range for an account owned by our program.
    // The re-read of program_config is unreliable on busy devnet due to
    // concurrent account creation incrementing the index.
    let actual_settings = if signature.is_some() {
        let client = ctx.solana_client();
        let mut found = settings;
        for offset in 0..SPECULATION_RANGE {
            let (pda, _) = pda::find_settings(smart_account_index + offset);
            match client.get_account(&pda).await {
                Ok(account) => {
                    // Check if this account is owned by our program and has
                    // our fee_payer as a signer (member) in its data.
                    if account.owner == PROGRAM_ID && account.data.len() >= 82 {
                        // Settings struct: disc(8) + seed(16) + settingsAuthority(32) + threshold(2) + timeLock(4) + txIndex(8) + staleTxIndex(8) + archOpt(1+) + ...
                        // Check if settingsAuthority matches what we sent
                        let auth_bytes = &account.data[24..56];
                        if let Some(ref pk) = input.settings_authority {
                            if auth_bytes == pk.as_ref() {
                                found = pda;
                                tracing::info!(
                                    "create_smart_account: found our settings at offset {offset}: {pda}"
                                );
                                break;
                            }
                        } else {
                            // For autonomous accounts, check if signer is our fee_payer
                            // by scanning the data for the fee_payer pubkey
                            let fp_bytes = input.fee_payer.pubkey().to_bytes();
                            if account.data.windows(32).any(|w| w == fp_bytes) {
                                found = pda;
                                tracing::info!(
                                    "create_smart_account: found our settings (autonomous) at offset {offset}: {pda}"
                                );
                                break;
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        found
    } else {
        settings
    };

    Ok(Output {
        signature,
        program_config,
        settings: actual_settings,
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
