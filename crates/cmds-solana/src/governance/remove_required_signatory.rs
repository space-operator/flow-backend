use std::str::FromStr;

use solana_sdk::instruction::AccountMeta;
use tracing_log::log::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "remove_required_signatory";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/remove_required_signatory.json");
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
    #[serde(with = "value::keypair")]
    pub governance: Keypair,
    #[serde(with = "value::pubkey")]
    pub signatory: Pubkey,
    #[serde(with = "value::pubkey")]
    pub beneficiary: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn remove_required_signatory(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    signatory: &Pubkey,
    beneficiary: &Pubkey,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"required-signatory".as_ref(),
        governance.as_ref(),
        signatory.as_ref(),
    ];
    let required_signatory_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("Required signatory address: {}", required_signatory_address);

    let accounts = vec![
        AccountMeta::new(*governance, true),
        AccountMeta::new(required_signatory_address, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let data = GovernanceInstruction::RemoveRequiredSignatory;

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, required_signatory_address)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, required_signatory_address) = remove_required_signatory(
        &program_id,
        &input.governance.pubkey(),
        &input.signatory,
        &input.beneficiary,
    );

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.governance.clone_keypair(),
        ]
        .into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "required_signatory_address" => required_signatory_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
