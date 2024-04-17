use std::str::FromStr;

use solana_sdk:: instruction::AccountMeta;
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "relinquish_token_owner_record_locks";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/relinquish_token_owner_record_locks.json");
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
    #[serde(with = "value::keypair::opt")]
    pub token_owner_record_lock_authority: Option<Keypair>,
    pub lock_ids: Option<Vec<u8>>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn relinquish_token_owner_record_locks(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    token_owner_record: &Pubkey,
    token_owner_record_lock_authority: Option<Pubkey>,
    // Args
    lock_ids: Option<Vec<u8>>,
) -> (Instruction, Pubkey) {
    let seeds: [&[u8]; 2] = [b"realm-config", realm.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("realm_config_address: {:?}", realm_config_address);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(realm_config_address, false),
        AccountMeta::new(*token_owner_record, false),
    ];

    if let Some(token_owner_record_lock_authority) = token_owner_record_lock_authority {
        accounts.push(AccountMeta::new_readonly(
            token_owner_record_lock_authority,
            true,
        ));
    }

    let data = GovernanceInstruction::RelinquishTokenOwnerRecordLocks { lock_ids };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, realm_config_address)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, realm_config_address) = relinquish_token_owner_record_locks(
        &program_id,
        &input.realm,
        &input.token_owner_record,
        input
            .token_owner_record_lock_authority
            .as_ref()
            .map(|k| k.pubkey()),
        input.lock_ids,
    );

    let signers = match input.token_owner_record_lock_authority {
        Some(token_owner_record_lock_authority) => vec![
            input.fee_payer.clone_keypair(),
            token_owner_record_lock_authority.clone_keypair(),
        ],
        None => vec![input.fee_payer.clone_keypair()],
    };
    
    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: signers.into(),
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
