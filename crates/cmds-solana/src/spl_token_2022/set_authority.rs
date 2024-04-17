use spl_token_2022::instruction::AuthorityType;

use crate::prelude::*;

const NAME: &str = "set_authority";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/set_authority.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub owned_pubkey: Pubkey,
    #[serde(with = "value::pubkey::opt")]
    pub new_authority_pubkey: Option<Pubkey>,
    pub authority_type: AuthorityType,
    #[serde(with = "value::pubkey")]
    pub owner_pubkey: Pubkey,
    pub signer_pubkeys: Vec<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let ix = spl_token_2022::instruction::set_authority(
        &spl_token_2022::id(),
        &input.owned_pubkey,
        input.new_authority_pubkey.as_ref(),
        input.authority_type,
        &input.owner_pubkey,
        &input.signer_pubkeys.iter().collect::<Vec<_>>(),
    )?;

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone_keypair()].into(),
        instructions: [ix].into(),
    };
    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
