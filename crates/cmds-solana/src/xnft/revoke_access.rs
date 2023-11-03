use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

// Command Name
const REMOVE_ACCESS: &str = "revoke_access";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/xnft/revoke_access.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(REMOVE_ACCESS)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(REMOVE_ACCESS, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub xnft: Pubkey,
    #[serde(with = "value::pubkey")]
    pub wallet: Pubkey,
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

    // Access PDA
    let seeds = &[
        "access".as_ref(),
        input.wallet.as_ref(),
        input.xnft.as_ref(),
    ];
    let access = Pubkey::find_program_address(seeds, &xnft_program_id).0;

    let accounts = xnft::accounts::RevokeAccess {
        xnft: input.xnft,
        authority: input.authority.pubkey(),
        access,
        wallet: input.wallet,
    }
    .to_account_metas(None);

    let data = xnft::instruction::RevokeAccess {}.data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<xnft::accounts::RevokeAccess>())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.authority.clone_keypair()].into(),
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
