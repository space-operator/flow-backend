use super::CandyGuardData;
use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use mpl_core_candy_guard::client::args::MintV1;
use solana_program::{instruction::Instruction, system_program, sysvar};
use solana_program::{compute_budget::ComputeBudgetInstruction, pubkey::Pubkey};

// Command Name
const NAME: &str = "mint_candy_machine_core";

const DEFINITION: &str = flow_lib::node_definition!("nft/candy_machine_core/mint_core.json");

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
    #[serde(with = "value::pubkey")]
    pub authority: Pubkey,
    pub payer: Wallet,
    pub minter: Wallet,
    #[serde(default, with = "value::pubkey::opt")]
    pub owner: Option<Pubkey>,
    pub mint_account: Wallet,
    #[serde(with = "value::pubkey")]
    pub collection_mint: Pubkey,
    #[serde(with = "value::pubkey")]
    pub collection_update_authority: Pubkey,
    pub candy_guards: CandyGuardData,
    pub group_label: Option<String>,
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
    static CANDY_MACHINE_PROGRAM_ID: Pubkey = mpl_core_candy_machine_core::ID;
    static CANDY_GUARD_PROGRAM_ID: Pubkey = mpl_core_candy_guard::ID;
    static MPL_CORE_PROGRAM_ID: Pubkey = mpl_core::ID;

    // Authority PDA
    let seeds = &["candy_machine".as_ref(), input.candy_machine.as_ref()];
    let candy_machine_authority_pda =
        Pubkey::find_program_address(seeds, &CANDY_MACHINE_PROGRAM_ID).0;

    // Candy Guard PDA
    let seeds = &["candy_guard".as_ref(), input.candy_machine.as_ref()];
    let candy_guard = Pubkey::find_program_address(seeds, &CANDY_GUARD_PROGRAM_ID).0;

    let accounts = mpl_core_candy_guard::client::accounts::MintV1 {
        candy_guard,
        candy_machine: input.candy_machine,
        candy_machine_authority_pda,
        payer: input.payer.pubkey(),
        minter: input.minter.pubkey(),
        owner: input.owner,
        asset: input.mint_account.pubkey(),
        collection: input.collection_mint,
        mpl_core_program: MPL_CORE_PROGRAM_ID,
        candy_machine_program: CANDY_MACHINE_PROGRAM_ID,
        system_program: system_program::ID,
        sysvar_instructions: sysvar::instructions::ID,
        recent_slothashes: sysvar::slot_hashes::ID,
    }
    .to_account_metas(None);

    // serialize input.candy_guards
    let data: mpl_core_candy_guard::types::CandyGuardData = input.candy_guards.into();
    let mut serialized_data = vec![0; data.size()];
    data.save(&mut serialized_data)?;

    let data = MintV1 {
        mint_args: serialized_data,
        label: input.group_label,
    }
    .data();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer,
            input.minter,
            // input.mint_account,
        ]
        .into(),
        instructions: [
            ComputeBudgetInstruction::set_compute_unit_limit(1_000_000u32),
            Instruction {
                program_id: CANDY_GUARD_PROGRAM_ID,
                accounts,
                data,
            },
        ]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
