use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "add_signatory";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/add_signatory.jsonc");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,
    pub governance_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub signatory: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// v3.1.2 account layout:
/// 0. `[writable]` Proposal account
/// 1. `[]` TokenOwnerRecord account of the Proposal owner
/// 2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
/// 3. `[writable]` Signatory Record Account
/// 4. `[signer]` Payer
/// 5. `[]` System program
pub fn add_signatory(
    program_id: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    signatory: &Pubkey,
) -> (Instruction, Pubkey) {
    let seeds: [&[u8]; 3] = [b"governance", proposal.as_ref(), signatory.as_ref()];
    let signatory_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(signatory_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

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

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, signatory_record_address) = add_signatory(
        &program_id,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.fee_payer.pubkey(),
        &input.signatory,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance_authority].into(),
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

#[cfg(test)]
mod tests {
    use super::super::SPL_GOVERNANCE_ID;
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_builder() {
        let proposal = Pubkey::new_unique();
        let token_owner_record = Pubkey::new_unique();
        let governance_authority = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let signatory = Pubkey::new_unique();

        let (ix, _addr) = add_signatory(
            &SPL_GOVERNANCE_ID,
            &proposal,
            &token_owner_record,
            &governance_authority,
            &payer,
            &signatory,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 6);
    }
}
