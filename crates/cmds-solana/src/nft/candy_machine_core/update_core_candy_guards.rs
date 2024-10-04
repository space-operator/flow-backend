use super::CandyGuardData;
use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;

use mpl_core_candy_guard::instruction::Update;

const NAME: &str = "update_core_candy_guards";

const DEFINITION: &str =
    flow_lib::node_definition!("nft/candy_machine_core/update_core_candy_guards.json");

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
    pub authority: Keypair,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub candy_machine: Pubkey,
    pub candy_guards: CandyGuardData,
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
    let candy_guard_program = mpl_core_candy_guard::ID;

    let seeds = &["candy_guard".as_ref(), input.candy_machine.as_ref()];
    let candy_guard = Pubkey::find_program_address(seeds, &candy_guard_program).0;

    let data: mpl_core_candy_guard::state::CandyGuardData = input.candy_guards.into();
    let mut serialized_data = vec![0; data.size()];
    data.save(&mut serialized_data)?;

    let accounts = mpl_core_candy_guard::accounts::Update {
        authority: input.authority.pubkey(),
        candy_guard,
        payer: input.payer.pubkey(),
        system_program: solana_program::system_program::ID,
    }
    .to_account_metas(None);

    let data = Update {
        data: serialized_data,
    }
    .data();

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.authority.clone_keypair()].into(),
        instructions: [Instruction {
            program_id: candy_guard_program,
            accounts,
            data,
        }]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
