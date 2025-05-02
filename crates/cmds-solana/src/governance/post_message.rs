use std::str::FromStr;

use solana_program::{instruction::AccountMeta, system_program};

use crate::prelude::*;

use super::{
    GovernanceChatInstruction, MessageBody, SPL_GOVERNANCE_CHAT_ID, SPL_GOVERNANCE_ID,
    with_realm_config_accounts,
};

const NAME: &str = "governance_post_message";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/post_message.json");
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
    pub governance: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,

    pub governance_authority: Wallet,

    pub chat_message: Wallet,
    pub body: MessageBody,
    #[serde(default, with = "value::pubkey::opt")]
    pub reply_to: Option<Pubkey>,
    #[serde(default, with = "value::pubkey::opt")]
    pub voter_weight_record: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn post_message(
    program_id: &Pubkey,
    // Accounts
    governance_program_id: &Pubkey,
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    reply_to: Option<Pubkey>,
    chat_message: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    body: MessageBody,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance_program_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*chat_message, true),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let is_reply = if let Some(reply_to) = reply_to {
        accounts.push(AccountMeta::new_readonly(reply_to, false));
        true
    } else {
        false
    };

    with_realm_config_accounts(
        governance_program_id,
        &mut accounts,
        realm,
        voter_weight_record,
        None,
    );

    let instruction = GovernanceChatInstruction::PostMessage { body, is_reply };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();
    let chat_program_id = Pubkey::from_str(SPL_GOVERNANCE_CHAT_ID).unwrap();

    let ix = post_message(
        &chat_program_id,
        &program_id,
        &input.realm,
        &input.governance,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        input.reply_to,
        &input.chat_message.pubkey(),
        &input.fee_payer.pubkey(),
        input.voter_weight_record,
        input.body,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer,
            input.governance_authority,
            input.chat_message,
        ]
        .into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
