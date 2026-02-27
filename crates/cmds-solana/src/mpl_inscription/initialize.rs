use anchor_lang::prelude::AccountMeta;
use borsh::{BorshDeserialize, BorshSerialize};
use rand::Rng;
use solana_sdk_ids::system_program;
use tracing::info;

use crate::{mpl_inscription::INSCRIPTION_PROGRAM_ID, prelude::*};

// Command Name
const NAME: &str = "initialize_inscription";

const DEFINITION: &str = flow_lib::node_definition!("mpl_inscription/initialize_inscription.jsonc");

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
    pub fee_payer: Wallet,
    pub inscription_account: Wallet,
    pub authority: Option<Wallet>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct InitializeInstructionData {
    discriminator: u8,
}

//    /// Initialize the Inscription and Metadata accounts.
//    #[account(0, writable, signer, name="inscription_account", desc = "The account where data is stored.")]
//    #[account(1, writable, name="inscription_metadata_account", desc = "The account to store the inscription account's metadata in.")]
//    #[account(2, writable, name="inscription_shard_account", desc="The shard account for the inscription counter.")]
//    #[account(3, writable, signer, name="payer", desc="The account that will pay for the rent.")]
//    #[account(4, optional, signer, name="authority", desc="The authority of the inscription account.")]
//    #[account(5, name="system_program", desc = "System program")]
fn create_initialize_inscription_instruction(
    inscription_account: Pubkey,
    inscription_metadata_account: Pubkey,
    inscription_shard_account: Pubkey,
    payer: Pubkey,
    authority: Option<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(inscription_account, true),
        AccountMeta::new(inscription_metadata_account, false),
        AccountMeta::new(inscription_shard_account, false),
        AccountMeta::new(payer, true),
    ];

    if let Some(authority) = authority {
        accounts.push(AccountMeta::new_readonly(authority, true));
    } else {
        accounts.push(AccountMeta::new_readonly(INSCRIPTION_PROGRAM_ID, false));
    }

    accounts.push(AccountMeta::new_readonly(system_program::ID, false));

    let data = InitializeInstructionData { discriminator: 0 }
        .try_to_vec()
        .unwrap();

    Instruction {
        program_id: INSCRIPTION_PROGRAM_ID,
        accounts,
        data,
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // random between 0-31
    let random_shard: u8 = rand::thread_rng().gen_range(0..32);
    info!("random_shard: {}", random_shard);

    let inscription_shard_account = Pubkey::find_program_address(
        &[
            "Inscription".as_bytes(),
            "Shard".as_bytes(),
            INSCRIPTION_PROGRAM_ID.as_ref(),
            random_shard.to_le_bytes().as_ref(),
        ],
        &INSCRIPTION_PROGRAM_ID,
    )
    .0;

    info!("inscription_shard_account: {}", inscription_shard_account);

    let inscription_metadata_account = Pubkey::find_program_address(
        &[
            "Inscription".as_bytes(),
            INSCRIPTION_PROGRAM_ID.as_ref(),
            input.inscription_account.pubkey().as_ref(),
        ],
        &INSCRIPTION_PROGRAM_ID,
    )
    .0;

    info!(
        "inscription_metadata_account: {}",
        inscription_metadata_account
    );

    let instruction = create_initialize_inscription_instruction(
        input.inscription_account.pubkey(),
        inscription_metadata_account,
        inscription_shard_account,
        input.fee_payer.pubkey(),
        input.authority.as_ref().map(|a| a.pubkey()),
    );

    info!("instruction: {:?}", instruction);

    let signer = if let Some(authority) = &input.authority {
        vec![&input.fee_payer, &input.inscription_account, authority]
    } else {
        vec![&input.fee_payer, &input.inscription_account]
    };
    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: signer.into_iter().cloned().collect(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "inscription_shard_account" => inscription_shard_account,
                "inscription_metadata_account" => inscription_metadata_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
