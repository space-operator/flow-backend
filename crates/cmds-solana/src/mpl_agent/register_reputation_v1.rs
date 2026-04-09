use crate::prelude::*;

use super::{find_agent_reputation_pda, to_instruction_v3, to_pubkey_v2};
use mpl_agent_reputation::instructions::RegisterReputationV1Builder;

const NAME: &str = "register_reputation_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_agent/register_reputation_v1.jsonc");

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
    pub asset: Pubkey,
    #[serde(default)]
    pub authority: Option<Wallet>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub agent_reputation: Pubkey,
}

pub async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let authority_pk = input
        .authority
        .as_ref()
        .map_or_else(|| input.fee_payer.pubkey(), |w| w.pubkey());

    let (agent_reputation, _) = find_agent_reputation_pda(&input.asset);

    let mut builder = RegisterReputationV1Builder::new();
    builder
        .agent_reputation(to_pubkey_v2(&agent_reputation))
        .asset(to_pubkey_v2(&input.asset))
        .payer(to_pubkey_v2(&input.fee_payer.pubkey()))
        .authority(Some(to_pubkey_v2(&authority_pk)));

    if let Some(collection) = input.collection {
        builder.collection(Some(to_pubkey_v2(&collection)));
    }

    let instruction = to_instruction_v3(builder.instruction());

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
                "agent_reputation" => agent_reputation,
            },
        )
        .await?
        .signature;

    Ok(Output {
        signature,
        agent_reputation,
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
