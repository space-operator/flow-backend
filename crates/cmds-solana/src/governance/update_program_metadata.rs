use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, get_program_metadata_address};

const NAME: &str = "update_program_metadata";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/update_program_metadata.jsonc");
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
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// v3.1.2 account layout:
/// 0. `[writable]` Program Metadata account (PDA: ['metadata'])
/// 1. `[signer]` Payer
/// 2. `[]` System program
pub fn update_program_metadata(program_id: &Pubkey, payer: &Pubkey) -> (Instruction, Pubkey) {
    let program_metadata_address = get_program_metadata_address(program_id);

    let accounts = vec![
        AccountMeta::new(program_metadata_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = GovernanceInstruction::UpdateProgramMetadata {};

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, program_metadata_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, program_metadata_address) =
        update_program_metadata(&program_id, &input.fee_payer.pubkey());

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "program_metadata_address" => program_metadata_address,
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
        let payer = Pubkey::new_unique();

        let (ix, _addr) = update_program_metadata(&SPL_GOVERNANCE_ID, &payer);

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 3);
    }
}
