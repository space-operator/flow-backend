use std::str::FromStr;

use solana_program::instruction::AccountMeta;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "withdraw_governing_tokens";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/withdraw_governing_tokens.json");
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
    #[serde(with = "value::pubkey")]
    pub governing_token_destination: Pubkey,

    pub governing_token_owner: Wallet,
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

pub fn withdraw_governing_tokens(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_destination: &Pubkey,
    governing_token_owner: &Pubkey,
    // Args
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
        AccountMeta::new(*governing_token_destination, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(realm_config_address, false),
    ];

    let data = GovernanceInstruction::WithdrawGoverningTokens {};

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

    let (ix, realm_config_address, governing_token_holding_address, token_owner_record_address) =
        withdraw_governing_tokens(
            &program_id,
            &input.realm,
            &input.governing_token_destination,
            &input.governing_token_owner.pubkey(),
            &input.governing_token_mint,
        );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governing_token_owner].into(),
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
