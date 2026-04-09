use crate::prelude::*;

use super::{
    find_execution_delegate_record_pda, find_executive_profile_pda, to_instruction_v3, to_pubkey_v2,
};
use mpl_agent_tools::instructions::RevokeExecutionV1Builder;

const NAME: &str = "revoke_execution_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_agent/revoke_execution_v1.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub executive_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub agent_asset: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub destination: Option<Pubkey>,
    #[serde(default)]
    pub authority: Option<Wallet>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub execution_delegate_record: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let authority_pk = input
        .authority
        .as_ref()
        .map_or_else(|| input.fee_payer.pubkey(), |w| w.pubkey());

    let destination = input
        .destination
        .unwrap_or_else(|| input.fee_payer.pubkey());

    let (executive_profile, _) = find_executive_profile_pda(&input.executive_authority);
    let (execution_delegate_record, _) =
        find_execution_delegate_record_pda(&executive_profile, &input.agent_asset);

    let instruction = RevokeExecutionV1Builder::new()
        .execution_delegate_record(to_pubkey_v2(&execution_delegate_record))
        .agent_asset(to_pubkey_v2(&input.agent_asset))
        .destination(to_pubkey_v2(&destination))
        .payer(to_pubkey_v2(&input.fee_payer.pubkey()))
        .authority(Some(to_pubkey_v2(&authority_pk)))
        .instruction();
    let instruction = to_instruction_v3(instruction);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer]
            .into_iter()
            .chain(input.authority)
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx
        .execute(
            ins,
            value::map! {
                "execution_delegate_record" => execution_delegate_record,
            },
        )
        .await?
        .signature;

    Ok(Output {
        signature,
        execution_delegate_record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
