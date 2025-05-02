use std::str::FromStr;

use solana_program::{instruction::AccountMeta, system_program};
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "deposit_governing_tokens";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/deposit_governing_tokens.json");
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
    pub realm: Pubkey,
    pub governing_token_owner: Wallet,
    pub governing_token_source_authority: Wallet,
    pub amount: u64,
    #[serde(with = "value::pubkey")]
    pub governing_token_mint: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn deposit_governing_tokens(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_source: &Pubkey,
    governing_token_owner: &Pubkey,
    governing_token_source_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    amount: u64,
    governing_token_mint: &Pubkey,
) -> (Instruction, Pubkey, Pubkey, Pubkey) {
    let seeds = [
        b"governance",
        realm.as_ref(),
        governing_token_mint.as_ref(),
        governing_token_owner.as_ref(),
    ];
    let token_owner_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    let seeds = [b"governance", realm.as_ref(), governing_token_mint.as_ref()];
    let governing_token_holding_address = Pubkey::find_program_address(&seeds, program_id).0;

    let seeds = [b"realm-config", realm.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(*governing_token_source, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new_readonly(*governing_token_source_authority, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(realm_config_address, false),
    ];

    let data = GovernanceInstruction::DepositGoverningTokens { amount };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (
        instruction,
        realm_config_address,
        governing_token_holding_address,
        token_owner_record_address,
    )
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let governing_token_source = spl_associated_token_account::get_associated_token_address(
        &input.governing_token_owner.pubkey(),
        &input.governing_token_mint,
    );
    info!("governing_token_source: {governing_token_source}");

    let (ix, realm_config_address, governing_token_holding_address, token_owner_record_address) =
        deposit_governing_tokens(
            &program_id,
            &input.realm,
            &governing_token_source,
            &input.governing_token_owner.pubkey(),
            &input.governing_token_source_authority.pubkey(),
            &input.fee_payer.pubkey(),
            input.amount,
            &input.governing_token_mint,
        );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer,
            input.governing_token_owner,
            input.governing_token_source_authority,
        ]
        .into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "realm_config_address" => realm_config_address,
                "governing_token_holding_address" => governing_token_holding_address,
                "token_owner_record_address" => token_owner_record_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
