use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use mpl_candy_guard::instruction::MintV2;
use solana_program::{instruction::Instruction, system_program, sysvar};
use solana_sdk::{compute_budget::ComputeBudgetInstruction, pubkey::Pubkey};

use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    instructions::MetadataDelegateRole,
};

use super::CandyGuardData;

// Command Name
const MINT: &str = "mint";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/NFT/candy_machine/mint.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(MINT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(MINT, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub candy_machine: Pubkey,
    #[serde(with = "value::pubkey")]
    pub authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub minter: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::keypair")]
    pub mint_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub collection_mint: Pubkey,
    #[serde(with = "value::pubkey")]
    pub collection_update_authority: Pubkey,
    pub candy_guards: CandyGuardData,
    pub group_label: Option<String>,
    // Optional
    #[serde(default = "rule_set_default", with = "value::pubkey")]
    pub rule_set: Pubkey,
    #[serde(default = "rule_set_default", with = "value::pubkey")]
    pub authorization_rules_program: Pubkey,
    #[serde(default = "rule_set_default", with = "value::pubkey")]
    pub authorization_rules: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

fn rule_set_default() -> Pubkey {
    mpl_candy_machine_core::id()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let token_metadata_program = mpl_token_metadata::id();
    let candy_machine_program = mpl_candy_machine_core::id();
    let candy_guard_program = mpl_candy_guard::id();

    // Authority PDA
    let seeds = &["candy_machine".as_ref(), input.candy_machine.as_ref()];
    let candy_machine_authority_pda = Pubkey::find_program_address(seeds, &candy_machine_program).0;

    // Candy Guard PDA
    let seeds = &["candy_guard".as_ref(), input.candy_machine.as_ref()];
    let candy_guard = Pubkey::find_program_address(seeds, &candy_guard_program).0;

    // Metadata PDA
    let nft_metadata = Metadata::find_pda(&input.mint_account).0;

    // Master Edition PDA
    let nft_master_edition = MasterEdition::find_pda(&input.mint_account).0;

    // NFT Associated Token Account
    let nft_associated_token_account = spl_associated_token_account::get_associated_token_address(
        &input.minter.pubkey(),
        &input.mint_account,
    );

    // Metadata TokenRecord Account
    let nft_token_record = mpl_token_metadata::pda::find_token_record_account(
        &input.mint_account,
        &nft_associated_token_account,
    )
    .0;

    // Collection Delegate Record PDA
    let collection_delegate_record =
        mpl_token_metadata::pda::find_metadata_delegate_record_account(
            &input.collection_mint,
            MetadataDelegateRole::Collection,
            &input.collection_update_authority,
            &candy_machine_authority_pda,
        )
        .0;

    // Collection Metadata PDA
    let collection_metadata = Metadata::find_pda(&input.collection_mint).0;

    // Collection Master Edition PDA
    let collection_master_edition = MasterEdition::find_pda(&input.collection_mint).0;

    let accounts = mpl_candy_guard::accounts::MintV2 {
        candy_guard,
        candy_machine_program,
        candy_machine: input.candy_machine,
        candy_machine_authority_pda,
        payer: input.payer.pubkey(),
        minter: input.minter.pubkey(),
        nft_mint: input.mint_account,
        nft_mint_authority: input.mint_authority.pubkey(),
        nft_metadata,
        nft_master_edition,
        token: Some(nft_associated_token_account),
        token_record: Some(nft_token_record),
        collection_delegate_record,
        collection_mint: input.collection_mint,
        collection_metadata,
        collection_master_edition,
        collection_update_authority: input.collection_update_authority,
        token_metadata_program,
        spl_token_program: spl_token::id(),
        spl_ata_program: Some(spl_associated_token_account::id()),
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
        recent_slothashes: sysvar::slot_hashes::id(),
        authorization_rules_program: Some(mpl_candy_guard::id()),
        authorization_rules: Some(mpl_candy_guard::id()),
    }
    .to_account_metas(None);

    // serialize input.candy_guards
    let data: mpl_candy_guard::state::CandyGuardData = input.candy_guards.into();
    let mut serialized_data = vec![0; data.size()];
    data.save(&mut serialized_data)?;

    let data = MintV2 {
        mint_args: serialized_data,
        label: input.group_label,
    }
    .data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_candy_machine_core::accounts::MintV2,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.minter.clone_keypair(),
            input.mint_authority.clone_keypair(),
        ]
        .into(),
        instructions: [
            ComputeBudgetInstruction::set_compute_unit_limit(1_000_000u32),
            Instruction {
                program_id: mpl_candy_guard::id(),
                accounts,
                data,
            },
        ]
        .into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
