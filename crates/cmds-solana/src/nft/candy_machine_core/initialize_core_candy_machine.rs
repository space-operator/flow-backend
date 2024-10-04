use super::CandyMachineData as CandyMachineDataAlias;
use crate::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, system_instruction, system_program};
use solana_sdk::pubkey::Pubkey;

use mpl_core_candy_machine_core::{instruction::Initialize, CandyMachineData};

const NAME: &str = "initialize_candy_machine_core";

const DEFINITION: &str =
    flow_lib::node_definition!("nft/candy_machine_core/initialize_candy_machine_core.json");

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
    #[serde(with = "value::keypair")]
    pub candy_machine: Keypair,
    #[serde(with = "value::pubkey")]
    pub authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub collection_mint: Pubkey,
    #[serde(with = "value::keypair")]
    pub collection_update_authority: Keypair,
    pub candy_machine_data: CandyMachineDataAlias,
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
    let candy_machine_program = mpl_core_candy_machine_core::id();
    let mpl_core_program = mpl_core::ID;
    let candy_pubkey = input.candy_machine.pubkey();

    // Authority PDA
    let seeds = &["candy_machine".as_ref(), candy_pubkey.as_ref()];
    let authority_pda = Pubkey::find_program_address(seeds, &candy_machine_program).0;

    let candy_machine_data = CandyMachineData::from(input.candy_machine_data);

    let accounts = mpl_core_candy_machine_core::accounts::Initialize {
        candy_machine: candy_pubkey,
        authority_pda,
        authority: input.authority,
        payer: input.payer.pubkey(),
        collection: input.collection_mint,
        collection_update_authority: input.collection_update_authority.pubkey(),
        system_program: system_program::ID,
        sysvar_instructions: solana_program::sysvar::instructions::id(),
        mpl_core_program,
    }
    .to_account_metas(None);

    let data = Initialize {
        data: candy_machine_data.clone(),
    }
    .data();

    // TODO check size
    let candy_account_size = candy_machine_data.get_space_for_candy().unwrap_or(216);

    let lamports = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(candy_account_size)
        .await?;

    let create_ix = system_instruction::create_account(
        &input.payer.pubkey(),
        &candy_pubkey,
        lamports,
        candy_account_size as u64,
        &candy_machine_program,
    );

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.candy_machine.clone_keypair(),
            input.collection_update_authority.clone_keypair(),
        ]
        .into(),
        instructions: [
            create_ix,
            Instruction {
                program_id: candy_machine_program,
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
