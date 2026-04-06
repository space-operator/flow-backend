use crate::prelude::*;

use super::{find_executive_profile_pda, to_instruction_v3, to_pubkey_v2};
use mpl_agent_tools::instructions::RegisterExecutiveV1Builder;

const NAME: &str = "register_executive_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_agent/register_executive_v1.jsonc");

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
    pub executive_profile: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let authority_pk = input
        .authority
        .as_ref()
        .map_or_else(|| input.fee_payer.pubkey(), |w| w.pubkey());

    let (executive_profile, _) = find_executive_profile_pda(&authority_pk);

    let instruction = RegisterExecutiveV1Builder::new()
        .executive_profile(to_pubkey_v2(&executive_profile))
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
                "executive_profile" => executive_profile,
            },
        )
        .await?
        .signature;

    Ok(Output {
        signature,
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
