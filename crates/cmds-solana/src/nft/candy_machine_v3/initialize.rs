use crate::{
    nft::{CandyMachineDataAlias, TokenStandard},
    prelude::*,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, system_instruction, };
use solana_sdk_ids::system_program;
use solana_program::pubkey::Pubkey;

use mpl_candy_machine_core::{instruction::InitializeV2, CandyMachineData};
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    types::MetadataDelegateRole,
};

// Command Name
const INITIALIZE_CANDY_MACHINE: &str = "initialize_candy_machine";

const DEFINITION: &str = flow_lib::node_definition!("nft/candy_machine/initialize.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(INITIALIZE_CANDY_MACHINE)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(INITIALIZE_CANDY_MACHINE, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub candy_machine: Wallet,
    #[serde(with = "value::pubkey")]
    pub authority: Pubkey,
    pub payer: Wallet,
    #[serde(with = "value::pubkey")]
    pub collection_mint: Pubkey,
    pub collection_update_authority: Wallet,
    pub candy_machine_data: CandyMachineDataAlias,
    pub token_standard: TokenStandard,
    // Optional
    #[serde(default = "value::default::bool_true")]
    submit: bool,
    #[serde(default = "rule_set_default", with = "value::pubkey")]
    pub rule_set: Pubkey,
    #[serde(default = "rule_set_default", with = "value::pubkey")]
    pub authorization_rules_program: Pubkey,
    #[serde(default = "rule_set_default", with = "value::pubkey")]
    pub authorization_rules: Pubkey,
}

fn rule_set_default() -> Pubkey {
    mpl_candy_machine_core::id()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let token_metadata_program = mpl_token_metadata::ID;
    let candy_machine_program = mpl_candy_machine_core::id();
    let candy_pubkey = input.candy_machine.pubkey();

    // Authority PDA
    let seeds = &["candy_machine".as_ref(), candy_pubkey.as_ref()];
    let authority_pda = Pubkey::find_program_address(seeds, &candy_machine_program).0;

    // Collection Metadata PDA
    let collection_metadata = Metadata::find_pda(&input.collection_mint).0;

    // Master Edition PDA
    let collection_master_edition = MasterEdition::find_pda(&input.collection_mint).0;

    // Collection Delegate Record PDA
    let collection_delegate_record =
        mpl_token_metadata::accounts::MetadataDelegateRecord::find_pda(
            &input.collection_mint,
            MetadataDelegateRole::Collection,
            &input.collection_update_authority.pubkey(),
            &authority_pda,
        )
        .0;

    let candy_machine_data = CandyMachineData::from(input.candy_machine_data);

    let accounts = mpl_candy_machine_core::accounts::InitializeV2 {
        candy_machine: candy_pubkey,
        authority_pda,
        authority: input.authority,
        payer: input.payer.pubkey(),
        rule_set: Some(input.rule_set),
        collection_metadata,
        collection_mint: input.collection_mint,
        collection_master_edition,
        collection_update_authority: input.collection_update_authority.pubkey(),
        collection_delegate_record,
        token_metadata_program,
        system_program: system_program::ID,
        sysvar_instructions: solana_program::sysvar::instructions::id(),
        authorization_rules_program: Some(input.authorization_rules_program),
        authorization_rules: Some(input.authorization_rules),
    }
    .to_account_metas(None);

    let token_standard = input.token_standard as u8;

    let data = InitializeV2 {
        data: candy_machine_data.clone(),
        token_standard,
    }
    .data();

    // TODO check size
    let candy_account_size = candy_machine_data.get_space_for_candy().unwrap_or(216);

    let lamports = ctx
        .solana_client()
        .get_minimum_balance_for_rent_exemption(candy_account_size)
        .await?;

    let create_ix = system_instruction::create_account(
        &input.payer.pubkey(),
        &input.candy_machine.pubkey(),
        lamports,
        candy_account_size as u64,
        &mpl_candy_machine_core::id(),
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer,
            input.candy_machine,
            input.collection_update_authority,
        ]
        .into(),
        instructions: [
            create_ix,
            Instruction {
                program_id: mpl_candy_machine_core::id(),
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

// {
//     "items_available": 10,
//     "symbol": "CORE",
//     "seller_fee_basis_points": 500,
//     "max_supply": 0,
//     "is_mutable": true,
//     "creators": [
//       {
//         "address": "2gdutJtCz1f9P3NJGP4HbBYFCHMh8rVAhmT2QDSb9dN9",
//         "verified": false,
//         "share": 100
//       }],
//     "config_line_settings": {
//       "prefix_name": "TEST",
//       "name_length": 10,
//       "prefix_uri": "https://arweave.net/",
//       "uri_length": 50,
//       "is_sequential": false
//     },
//     "hiddenSettings": null
//   }
