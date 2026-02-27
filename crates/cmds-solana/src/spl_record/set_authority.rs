use crate::prelude::*;

use spl_record::instruction as record_instruction;

const NAME: &str = "set_authority_record";

const DEFINITION: &str = flow_lib::node_definition!("spl_record/set_authority.jsonc");

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    record_account: Pubkey,
    current_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    new_authority: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let instruction = record_instruction::set_authority(
        &input.record_account,
        &input.current_authority.pubkey(),
        &input.new_authority,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.current_authority].into(),
        instructions: [instruction].into(),
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
