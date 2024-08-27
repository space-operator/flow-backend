use std::str::FromStr;

use solana_sdk::instruction::AccountMeta;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "execute_transaction";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/execute_transaction.json");
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
    pub proposal_transaction: Pubkey,
    #[serde(with = "value::pubkey")]
    pub instruction_program_id: Pubkey,
    pub instruction_accounts: Vec<AccountMeta>,
    // TODO workaround for testing
    pub additional_signers: Option<Vec<String>>,
    #[serde(with = "value::keypair::opt")]
    pub signer_1: Option<Keypair>,
    #[serde(with = "value::keypair::opt")]
    pub signer_2: Option<Keypair>,
    #[serde(with = "value::keypair::opt")]
    pub signer_3: Option<Keypair>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn execute_transaction(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_transaction: &Pubkey,
    instruction_program_id: &Pubkey,
    instruction_accounts: &[AccountMeta],
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_transaction, false),
        AccountMeta::new_readonly(*instruction_program_id, false),
    ];

    accounts.extend_from_slice(instruction_accounts);

    let instruction = GovernanceInstruction::ExecuteTransaction {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}
async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let ix = execute_transaction(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.proposal_transaction,
        &input.instruction_program_id,
        &input.instruction_accounts,
    );

    let mut signers: Vec<Keypair> = [input.fee_payer.clone_keypair()].into();

    match input.additional_signers {
        None => {
            if let Some(signer) = input.signer_1 {
                signers.push(signer.clone_keypair());
            }
            if let Some(signer) = input.signer_2 {
                signers.push(signer.clone_keypair());
            }
            if let Some(signer) = input.signer_3 {
                signers.push(signer.clone_keypair());
            }
        }
        Some(ref additional_signers) => {
            let additional_signers: Vec<Keypair> = additional_signers
                .iter()
                .map(|s| Keypair::from_base58_string(s).clone_keypair())
                .collect();
            signers.extend(additional_signers);
        }
    }

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers,
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
