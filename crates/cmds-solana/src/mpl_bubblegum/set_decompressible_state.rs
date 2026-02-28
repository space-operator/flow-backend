use super::DecompressibleState;
use crate::prelude::*;
use mpl_bubblegum::instructions::SetDecompressibleStateBuilder;
use solana_program::pubkey::Pubkey;

const NAME: &str = "set_decompressible_state";

const DEFINITION: &str = flow_lib::node_definition!("mpl_bubblegum/set_decompressible_state.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payer: Wallet,
    pub tree_creator: Wallet,
    #[serde(with = "value::pubkey")]
    pub merkle_tree: Pubkey,
    pub decompressable_state: DecompressibleState,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let tree_config = mpl_bubblegum::accounts::TreeConfig::find_pda(&input.merkle_tree).0;

    let ix = SetDecompressibleStateBuilder::new()
        .tree_config(tree_config)
        .tree_creator(input.tree_creator.pubkey())
        .decompressable_state(input.decompressable_state.into())
        .instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.tree_creator].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
