use std::str::FromStr;

use solana_sdk::instruction::AccountMeta;

use crate::prelude::*;

use super::{GovernanceInstruction, SetRealmAuthorityAction, SPL_GOVERNANCE_ID};

const NAME: &str = "set_realm_authority";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/set_realm_authority.json");
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

    pub realm_authority: Wallet,
    #[serde(with = "value::pubkey::opt")]
    pub new_realm_authority: Option<Pubkey>,
    pub action: SetRealmAuthorityAction,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn set_realm_authority(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    realm_authority: &Pubkey,
    new_realm_authority: Option<&Pubkey>,
    // Args
    action: SetRealmAuthorityAction,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*realm, false),
        AccountMeta::new_readonly(*realm_authority, true),
    ];

    match action {
        SetRealmAuthorityAction::SetChecked | SetRealmAuthorityAction::SetUnchecked => {
            accounts.push(AccountMeta::new_readonly(
                *new_realm_authority.unwrap(),
                false,
            ));
        }
        SetRealmAuthorityAction::Remove => {}
    }

    let instruction = GovernanceInstruction::SetRealmAuthority { action };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let ix = set_realm_authority(
        &program_id,
        &input.realm,
        &input.realm_authority.pubkey(),
        input.new_realm_authority.as_ref(),
        input.action,
    );

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.realm_authority].into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
