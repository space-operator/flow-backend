use spl_token_2022::instruction::AuthorityType;

use crate::prelude::*;

const NAME: &str = "set_authority_2022";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/spl_token_2022/set_authority.json");
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
    pub owned_pubkey: Pubkey,
    #[serde(with = "value::pubkey::opt")]
    pub new_authority: Option<Pubkey>,
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

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ix = spl_token_2022::instruction::set_authority(
        &spl_token_2022::id(),
        &input.owned_pubkey,
        input.new_authority.as_ref(),
        input.authority_type,
        &input.owner_pubkey,
        &input.signer_pubkeys.iter().collect::<Vec<_>>(),
    )?;

    // TODO if signers not empty, add signers as signer

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [ix].into(),
    };
    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
