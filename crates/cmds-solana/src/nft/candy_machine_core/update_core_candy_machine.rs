use super::CandyMachineData as CandyMachineDataAlias;
use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;

use mpl_core_candy_machine_core::{instruction::Update, CandyMachineData};

const NAME: &str = "update_candy_machine_core";

const DEFINITION: &str =
    flow_lib::node_definition!("nft/candy_machine_core/update_candy_machine_core.json");

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
    #[serde(with = "value::keypair")]
    pub candy_machine: Keypair,
    #[serde(with = "value::keypair")]
    pub authority: Keypair,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    pub candy_machine_data: CandyMachineDataAlias,
    // Optional
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let candy_machine_program = mpl_core_candy_machine_core::id();
    let candy_pubkey = input.candy_machine.pubkey();

    let candy_machine_data = CandyMachineData::from(input.candy_machine_data);

    let accounts = mpl_core_candy_machine_core::accounts::Update {
        candy_machine: candy_pubkey,
        authority: input.authority.pubkey(),
    }
    .to_account_metas(None);

    let data = Update {
        data: candy_machine_data.clone(),
    }
    .data();

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.authority.clone_keypair()].into(),
        instructions: [Instruction {
            program_id: candy_machine_program,
            accounts,
            data,
        }]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
