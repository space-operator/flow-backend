use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, VoteType, with_realm_config_accounts};

const NAME: &str = "create_proposal";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/create_proposal.json");
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
    pub governance: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal_owner_record: Pubkey,

    pub governance_authority: Wallet,
    #[serde(default, with = "value::pubkey::opt")]
    pub voter_weight_record: Option<Pubkey>,
    #[serde(with = "value::pubkey")]
    pub realm: Pubkey,
    pub name: String,
    pub description_link: String,
    #[serde(with = "value::pubkey")]
    pub governing_token_mint: Pubkey,
    pub vote_type: VoteType,
    pub use_deny_option: bool,
    pub options: Vec<String>,
    #[serde(with = "value::pubkey")]
    pub proposal_seed: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn create_proposal(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    realm: &Pubkey,
    name: String,
    description_link: String,
    governing_token_mint: &Pubkey,
    vote_type: VoteType,
    options: Vec<String>,
    use_deny_option: bool,
    proposal_seed: &Pubkey,
) -> (Instruction, Pubkey, Pubkey) {
    let seeds = [
        b"governance",
        governance.as_ref(),
        governing_token_mint.as_ref(),
        proposal_seed.as_ref(),
    ];
    let proposal_address = Pubkey::find_program_address(&seeds, program_id).0;

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(proposal_address, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    with_realm_config_accounts(program_id, &mut accounts, realm, voter_weight_record, None);

    // Deposit is only required when there are more active proposal then the
    // configured exempt amount Note: We always pass the account because the
    // actual value is not known here without passing Governance account data
    let seeds = [
        b"proposal-deposit",
        proposal_address.as_ref(),
        payer.as_ref(),
    ];
    let proposal_deposit_address = Pubkey::find_program_address(&seeds, program_id).0;
    accounts.push(AccountMeta::new(proposal_deposit_address, false));

    let instruction = GovernanceInstruction::CreateProposal {
        name,
        description_link,
        vote_type,
        options,
        use_deny_option,
        proposal_seed: *proposal_seed,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    };

    (instruction, proposal_address, proposal_deposit_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, proposal_address, proposal_deposit_address) = create_proposal(
        &program_id,
        &input.governance,
        &input.proposal_owner_record,
        &input.governance_authority.pubkey(),
        &input.fee_payer.pubkey(),
        input.voter_weight_record,
        &input.realm,
        input.name,
        input.description_link,
        &input.governing_token_mint,
        input.vote_type,
        input.options,
        input.use_deny_option,
        &input.proposal_seed,
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
                "proposal_address" => proposal_address,
                "proposal_deposit_address" => proposal_deposit_address

            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
