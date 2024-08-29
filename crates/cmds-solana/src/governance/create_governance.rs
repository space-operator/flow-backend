use std::str::FromStr;

use solana_sdk::{instruction::AccountMeta, system_program};

use crate::prelude::*;

use super::{
    with_realm_config_accounts, GovernanceConfig, GovernanceInstruction, SPL_GOVERNANCE_ID,
};

const NAME: &str = "create_governance";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/create_governance.json");
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
    pub governance_seed: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,
    #[serde(with = "value::keypair")]
    pub create_authority: Keypair,
    #[serde(default, with = "value::pubkey::opt")]
    pub voter_weight_record: Option<Pubkey>,
    pub config: GovernanceConfig,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn create_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance_seed: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    create_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"account-governance",
        realm.as_ref(),
        governance_seed.as_ref(),
    ];
    let governance_address = Pubkey::find_program_address(&seeds, program_id).0;

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governance_address, false),
        AccountMeta::new_readonly(*governance_seed, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*create_authority, true),
    ];

    with_realm_config_accounts(program_id, &mut accounts, realm, voter_weight_record, None);

    let data = GovernanceInstruction::CreateGovernance { config };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, governance_address)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, governance_address) = create_governance(
        &program_id,
        &input.realm,
        &input.governance_seed,
        &input.token_owner_record,
        &input.fee_payer.pubkey(),
        &input.create_authority.pubkey(),
        input.voter_weight_record,
        input.config,
    );

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.create_authority.clone_keypair(),
        ]
        .into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "governance_address" => governance_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
