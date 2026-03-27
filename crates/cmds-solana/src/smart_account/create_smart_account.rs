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
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (program_config, _) = pda::find_program_config();

    // Read treasury from program config on-chain
    let client = ctx.solana_client();
    let config_data = client
        .get_account_data(&program_config)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to read program config: {e}")))?;

    // ProgramConfig layout (after 8-byte Anchor discriminator):
    // smart_account_index: u128 (16 bytes)
    // authority: Pubkey (32 bytes)
    // smart_account_creation_fee: u64 (8 bytes)
    // treasury: Pubkey (32 bytes)
    let treasury_offset = 8 + 16 + 32 + 8;
    if config_data.len() < treasury_offset + 32 {
        return Err(CommandError::msg("Program config data too short"));
    }
    let treasury = Pubkey::try_from(&config_data[treasury_offset..treasury_offset + 32])
        .map_err(|_| CommandError::msg("Invalid treasury pubkey"))?;

    let accounts = vec![
        AccountMeta::new(program_config, false),
        AccountMeta::new(treasury, false),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
    ];

    // Serialize args: CreateSmartAccountArgs
    let mut args_data = Vec::new();

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

    Ok(Output {
        signature,
        program_config,
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
