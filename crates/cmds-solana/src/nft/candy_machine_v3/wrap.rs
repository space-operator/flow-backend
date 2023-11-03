use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

// Command Name
const WRAP: &str = "wrap";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/NFT/candy_machine/wrap.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(WRAP)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(WRAP, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub candy_machine: Pubkey,
    #[serde(with = "value::keypair")]
    pub candy_machine_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub candy_guard: Pubkey,
    #[serde(with = "value::keypair")]
    pub candy_guard_authority: Keypair,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let accounts = mpl_candy_guard::accounts::Wrap {
        authority: input.candy_guard_authority.pubkey(),
        candy_machine: input.candy_machine,
        candy_machine_program: mpl_candy_machine_core::id(),
        candy_machine_authority: input.candy_machine_authority.pubkey(),
        candy_guard: input.candy_guard,
    }
    .to_account_metas(None);

    let data = mpl_candy_guard::instruction::Wrap {}.data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(
            std::mem::size_of::<mpl_candy_guard::accounts::Wrap>(),
        )
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.candy_guard_authority.clone_keypair(),
            input.candy_machine_authority.clone_keypair(),
        ]
        .into(),
        instructions: [Instruction {
            program_id: mpl_candy_guard::id(),
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
