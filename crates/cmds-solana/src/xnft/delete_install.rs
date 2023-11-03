use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

// Command Name
const DELETE_INSTALL: &str = "delete_install";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/xnft/delete_install.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(DELETE_INSTALL)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(DELETE_INSTALL, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub receiver: Pubkey,
    #[serde(with = "value::pubkey")]
    pub install: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let xnft_program_id = xnft::id();

    let accounts = xnft::accounts::DeleteInstall {
        install: input.install,
        receiver: input.receiver,
        authority: input.authority.pubkey(),
    }
    .to_account_metas(None);

    let data = xnft::instruction::DeleteInstall {}.data();

    let minimum_balance_for_rent_exemption =
        ctx.solana_client
            .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
                xnft::accounts::DeleteInstall,
            >())
            .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.authority.clone_keypair(), input.payer.clone_keypair()].into(),
        instructions: [Instruction {
            program_id: xnft_program_id,
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
