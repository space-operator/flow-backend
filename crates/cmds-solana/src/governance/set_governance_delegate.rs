use solana_program::instruction::AccountMeta;
use tracing::info;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "set_governance_delegate";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/set_governance_delegate.jsonc");
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

    pub governance_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub realm: Pubkey,
    #[serde(with = "value::pubkey")]
    pub governing_token_mint: Pubkey,
    #[serde(with = "value::pubkey")]
    pub governing_token_owner: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub new_governance_delegate: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}
pub fn set_governance_delegate(
    program_id: &Pubkey,
    // Accounts
    governance_authority: &Pubkey,
    // Args
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
    new_governance_delegate: &Option<Pubkey>,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"governance",
        realm.as_ref(),
        governing_token_mint.as_ref(),
        governing_token_owner.as_ref(),
    ];
    let vote_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    info!("vote_record_address: {:?}", vote_record_address);

    let accounts = vec![
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(vote_record_address, false),
    ];

    let data = GovernanceInstruction::SetGovernanceDelegate {
        new_governance_delegate: *new_governance_delegate,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };

    (instruction, vote_record_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, vote_record_address) = set_governance_delegate(
        &program_id,
        &input.governance_authority.pubkey(),
        &input.realm,
        &input.governing_token_mint,
        &input.governing_token_owner,
        &input.new_governance_delegate,
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
                "vote_record_address" => vote_record_address,
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
        let governance_authority = Pubkey::new_unique();
        let realm = Pubkey::new_unique();
        let governing_token_mint = Pubkey::new_unique();
        let governing_token_owner = Pubkey::new_unique();
        let new_governance_delegate: Option<Pubkey> = None;

        let (ix, _addr) = set_governance_delegate(
            &SPL_GOVERNANCE_ID,
            &governance_authority,
            &realm,
            &governing_token_mint,
            &governing_token_owner,
            &new_governance_delegate,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 2);
    }
}
