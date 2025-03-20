use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;
use super::ConfigLine;

// Command Name
const NAME: &str = "add_config_lines_core";

const DEFINITION: &str =
    flow_lib::node_definition!("nft/candy_machine_core/add_config_lines_core.json");

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
    #[serde(with = "value::pubkey")]
    pub candy_machine: Pubkey,
    pub authority: Wallet,
    pub payer: Wallet,
    pub index: u32,
    pub config_lines: Vec<ConfigLine>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let accounts = mpl_core_candy_machine_core::accounts::AddConfigLines {
        candy_machine: input.candy_machine,
        authority: input.authority.pubkey(),
    }
    .to_account_metas(None);

    let data = AddConfigLines {
        index: input.index,
        config_lines: input.config_lines.into_iter().map(Into::into).collect(),
    }
    .data();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.authority].into(),
        instructions: [Instruction {
            program_id: mpl_core_candy_machine_core::ID,
            accounts,
            data,
        }]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
