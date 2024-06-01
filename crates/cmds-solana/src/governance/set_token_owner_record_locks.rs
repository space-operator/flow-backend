use std::str::FromStr;

use solana_sdk::{clock::UnixTimestamp, instruction::AccountMeta, system_program};
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "set_token_owner_record_locks";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/set_token_owner_record_locks.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub realm: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,
    #[serde(with = "value::keypair")]
    pub token_owner_record_lock_authority: Keypair,
    pub lock_id: u8,
    pub expiry: Option<i64>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn set_token_owner_record_lock(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    token_owner_record: &Pubkey,
    token_owner_record_lock_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    lock_id: u8,
    expiry: Option<UnixTimestamp>,
) -> (Instruction, Pubkey) {
    let seeds: [&[u8]; 2] = [b"realm-config", realm.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("realm_config_address: {:?}", realm_config_address);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(realm_config_address, false),
        AccountMeta::new(*token_owner_record, false),
        AccountMeta::new_readonly(*token_owner_record_lock_authority, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = GovernanceInstruction::SetTokenOwnerRecordLock { lock_id, expiry };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, realm_config_address)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, realm_config_address) = set_token_owner_record_lock(
        &program_id,
        &input.realm,
        &input.token_owner_record,
        &input.token_owner_record_lock_authority.pubkey(),
        &input.fee_payer.pubkey(),
        input.lock_id,
        input.expiry,
    );

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.token_owner_record_lock_authority.clone_keypair(),
        ]
        .into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "realm_config_address" => realm_config_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
