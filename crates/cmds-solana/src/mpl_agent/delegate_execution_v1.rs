use crate::prelude::*;

use super::{
    find_agent_identity_pda, find_execution_delegate_record_pda, find_executive_profile_pda,
    to_instruction_v3, to_pubkey_v2,
};
use mpl_agent_tools::instructions::DelegateExecutionV1Builder;

const NAME: &str = "delegate_execution_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_agent/delegate_execution_v1.jsonc");

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
    /// Asset owner who authorizes the delegation (not the executive).
    /// Defaults to fee_payer if not provided.
    #[serde(default)]
    pub asset_authority: Option<Wallet>,
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
    #[serde_as(as = "AsPubkey")]
    pub executive_profile: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let authority_pk = input
        .asset_authority
        .as_ref()
        .map_or_else(|| input.fee_payer.pubkey(), |w| w.pubkey());

    let (executive_profile, _) = find_executive_profile_pda(&input.executive_authority);
    let (agent_identity, _) = find_agent_identity_pda(&input.agent_asset);
    let (execution_delegate_record, _) =
        find_execution_delegate_record_pda(&executive_profile, &input.agent_asset);

    let instruction = DelegateExecutionV1Builder::new()
        .executive_profile(to_pubkey_v2(&executive_profile))
        .agent_asset(to_pubkey_v2(&input.agent_asset))
        .agent_identity(to_pubkey_v2(&agent_identity))
        .execution_delegate_record(to_pubkey_v2(&execution_delegate_record))
        .payer(to_pubkey_v2(&input.fee_payer.pubkey()))
        .authority(Some(to_pubkey_v2(&authority_pk)))
        .instruction();
    let instruction = to_instruction_v3(instruction);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer]
            .into_iter()
            .chain(input.asset_authority)
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
                "executive_profile" => executive_profile,
            },
        )
        .await?
        .signature;

    Ok(Output {
        signature,
        execution_delegate_record,
        executive_profile,
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
