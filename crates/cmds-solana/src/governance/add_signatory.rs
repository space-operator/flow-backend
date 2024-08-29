use std::str::FromStr;

use solana_sdk::{instruction::AccountMeta, system_program};
use tracing::info;

use crate::prelude::*;

use super::{AddSignatoryAuthority, GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "add_signatory";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/add_signatory.json");
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
    pub governance: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub signatory: Pubkey,
    #[serde(default, with = "value::keypair::opt")]
    pub governance_authority: Option<Keypair>,
    pub add_signatory_authority: AddSignatoryAuthority,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn add_signatory(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    add_signatory_authority: &AddSignatoryAuthority,
    payer: &Pubkey,
    // Args
    signatory: &Pubkey,
) -> (Instruction, Pubkey) {
    let seeds: [&[u8]; 3] = [b"governance", proposal.as_ref(), signatory.as_ref()];
    let signatory_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(signatory_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    match add_signatory_authority {
        AddSignatoryAuthority::ProposalOwner {
            governance_authority,
            token_owner_record,
        } => {
            accounts.push(AccountMeta::new_readonly(*token_owner_record, false));
            //TODO add as signer
            accounts.push(AccountMeta::new_readonly(*governance_authority, true));
        }
        AddSignatoryAuthority::None => {
            let seeds = [
                b"required-signatory".as_ref(),
                governance.as_ref(),
                signatory.as_ref(),
            ];
            let required_signatory = Pubkey::find_program_address(&seeds, program_id).0;
            info!("required_signatory: {:?}", required_signatory);
            accounts.push(AccountMeta::new_readonly(required_signatory, false));
        }
    };

    let data = GovernanceInstruction::AddSignatory {
        signatory: *signatory,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, signatory_record_address)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, signatory_record_address) = add_signatory(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.add_signatory_authority,
        &input.fee_payer.pubkey(),
        &input.signatory,
    );

    let signers = match input.governance_authority {
        Some(governance_authority) => {
            vec![
                input.fee_payer.clone_keypair(),
                governance_authority.clone_keypair(),
            ]
        }
        None => vec![input.fee_payer.clone_keypair()],
    };
    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers,
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "signatory_record_address" => signatory_record_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
