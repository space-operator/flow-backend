use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, system_program};
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;

use super::CreateXnftParams;

// Command Name
const CREATE_XNFT: &str = "create_xnft";

const DEFINITION: &str = include_str!("../../../../node-definitions/solana/xnft/create_xnft.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(CREATE_XNFT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(CREATE_XNFT, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub publisher: Keypair,
    pub name: String,
    pub parameters: CreateXnftParams,
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
    let metadata_program = mpl_token_metadata::ID;

    // Master Mint PDA
    let seeds = &[
        "mint".as_ref(),
        input.authority.as_ref(),
        input.name.as_ref(),
    ];
    let master_mint = Pubkey::find_program_address(seeds, &xnft_program_id).0;

    // Master Token
    let master_token = get_associated_token_address(&input.authority, &master_mint);

    // xNFT PDA
    let seeds = &["xnft".as_ref(), master_mint.as_ref()];
    let xnft = Pubkey::find_program_address(seeds, &xnft_program_id).0;

    // Master Metadata
    let master_metadata = Pubkey::find_program_address(
        &[
            "metadata".as_ref(),
            metadata_program.to_bytes().as_ref(),
            master_mint.as_ref(),
        ],
        &metadata_program,
    )
    .0;

    let accounts = xnft::accounts::CreateAppXnft {
        master_mint,
        master_token,
        master_metadata,
        xnft,
        payer: input.payer.pubkey(),
        publisher: input.publisher.pubkey(),
        system_program: system_program::id(),
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        metadata_program,
        rent: solana_sdk::sysvar::rent::id(),
    }
    .to_account_metas(None);

    let params = xnft::state::CreateXnftParams::from(input.parameters);

    let data = xnft::instruction::CreateAppXnft {
        name: input.name,
        params,
    }
    .data();

    let minimum_balance_for_rent_exemption =
        ctx.solana_client
            .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
                xnft::accounts::CreateAppXnft,
            >())
            .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.publisher.clone_keypair()].into(),
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
                "master_mint"=>master_mint,
                "master_token"=>master_token,
                "master_metadata"=>master_metadata,
                "xnft"=>xnft,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
