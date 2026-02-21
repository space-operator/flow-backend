use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use mpl_candy_guard::instruction::Initialize;
use solana_program::{instruction::Instruction, };
use solana_sdk_ids::system_program;
use solana_program::pubkey::Pubkey;

// Command Name
const INITIALIZE_CANDY_GUARD: &str = "initialize_candy_guard";

const DEFINITION: &str =
    flow_lib::node_definition!("nft/candy_machine/initialize_candy_guard.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(INITIALIZE_CANDY_GUARD)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(INITIALIZE_CANDY_GUARD, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub base: Wallet,
    #[serde(with = "value::pubkey")]
    pub authority: Pubkey,
    pub payer: Wallet,
    pub candy_guards: super::CandyGuardData,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let candy_guard_program = mpl_candy_guard::id();

    let base_pubkey = input.base.pubkey();

    let seeds = &["candy_guard".as_ref(), base_pubkey.as_ref()];
    let candy_guard = Pubkey::find_program_address(seeds, &candy_guard_program).0;

    let accounts = mpl_candy_guard::accounts::Initialize {
        authority: input.authority,
        candy_guard,
        base: input.base.pubkey(),
        payer: input.payer.pubkey(),
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    // serialize input.candy_guards
    let data: mpl_candy_guard::state::CandyGuardData = input.candy_guards.into();
    let mut serialized_data = vec![0; data.size()];
    data.save(&mut serialized_data)?;

    let data = Initialize {
        data: serialized_data,
    }
    .data();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.base].into(),
        instructions: [Instruction {
            program_id: mpl_candy_guard::id(),
            accounts,
            data,
        }]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "candy_guard" => candy_guard,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
