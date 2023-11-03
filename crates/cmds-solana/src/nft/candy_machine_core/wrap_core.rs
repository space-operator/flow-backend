use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

// Command Name
const NAME: &str = "wrap_core";

const DEFINITION: &str = flow_lib::node_definition!("nft/candy_machine_core/wrap_core.json");

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
    pub candy_machine_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub candy_guard: Pubkey,
    pub candy_guard_authority: Wallet,
    pub payer: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    static CANDY_GUARD_PROGRAM_ID: Pubkey = mpl_core_candy_guard::ID;
    static CANDY_MACHINE_PROGRAM_ID: Pubkey = mpl_core_candy_machine_core::ID;

    let accounts = mpl_core_candy_guard::accounts::Wrap {
        authority: input.candy_guard_authority.pubkey(),
        candy_machine: input.candy_machine,
        candy_machine_program: CANDY_MACHINE_PROGRAM_ID,
        candy_machine_authority: input.candy_machine_authority.pubkey(),
        candy_guard: input.candy_guard,
    }
    .to_account_metas(None);

    let data = mpl_core_candy_guard::instruction::Wrap {}.data();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer,
            input.candy_guard_authority,
            input.candy_machine_authority,
        ]
        .into(),
        instructions: [Instruction {
            program_id: CANDY_GUARD_PROGRAM_ID,
            accounts,
            data,
        }]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
