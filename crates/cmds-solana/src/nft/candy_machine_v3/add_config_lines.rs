use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use mpl_candy_machine_core::instruction::AddConfigLines as MPLAddConfigLines;

use super::ConfigLine;

// Command Name
const ADD_CONFIG_LINES: &str = "add_config_lines";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/NFT/candy_machine/add_config_lines.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(ADD_CONFIG_LINES)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(ADD_CONFIG_LINES, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub candy_machine: Pubkey,
    #[serde(with = "value::keypair")]
    pub authority: Keypair,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
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
    let accounts = mpl_candy_machine_core::accounts::AddConfigLines {
        candy_machine: input.candy_machine,
        authority: input.authority.pubkey(),
    }
    .to_account_metas(None);

    let data = MPLAddConfigLines {
        index: input.index,
        config_lines: input.config_lines.into_iter().map(Into::into).collect(),
    }
    .data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_candy_machine_core::accounts::AddConfigLines,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.authority.clone_keypair()].into(),
        instructions: [Instruction {
            program_id: mpl_candy_machine_core::id(),
            accounts,
            data,
        }]
        .into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
