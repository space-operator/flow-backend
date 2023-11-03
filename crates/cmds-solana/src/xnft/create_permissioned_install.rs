use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, system_program};
use solana_sdk::pubkey::Pubkey;

// Command Name
const CREATE_PERMISSIONED_INSTALL: &str = "create_permissioned_install";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/xnft/create_permissioned_install.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(CREATE_PERMISSIONED_INSTALL)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(CREATE_PERMISSIONED_INSTALL, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub xnft: Pubkey,
    #[serde(with = "value::keypair")]
    pub target: Keypair,
    #[serde(with = "value::pubkey")]
    pub install_vault: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    //TODO
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let xnft_program_id = xnft::id();

    // Install PDA
    let authority = &input.authority.pubkey();
    let seeds = &["install".as_ref(), authority.as_ref(), input.xnft.as_ref()];
    let install = Pubkey::find_program_address(seeds, &xnft_program_id).0;

    // Access PDA
    let seeds = &["access".as_ref(), authority.as_ref(), input.xnft.as_ref()];
    let access = Pubkey::find_program_address(seeds, &xnft_program_id).0;

    let accounts = xnft::accounts::CreatePermissionedInstall {
        xnft: input.xnft,
        install_vault: input.install_vault,
        install,
        authority: input.authority.pubkey(),
        system_program: system_program::id(),
        access,
    }
    .to_account_metas(None);

    let data = xnft::instruction::CreatePermissionedInstall {}.data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            xnft::accounts::CreatePermissionedInstall,
        >())
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

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "install"=>install,
                "access"=>access,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
