use anchor_lang::prelude::AccountMeta;
use borsh::{BorshDeserialize, BorshSerialize};
// use rand::Rng;
// use solana_program::account_info::IntoAccountInfo;
use solana_sdk_ids::system_program;
use tracing::info;

use crate::{nft::inscriptions::INSCRIPTION_PROGRAM_ID, prelude::*};

// Command Name
const NAME: &str = "create_shard";

const DEFINITION: &str = flow_lib::node_definition!("nft/inscriptions/create_shard.jsonc");

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
    pub shard_number: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct CreateShardInstructionData {
    discriminator: u8,
}

impl CreateShardInstructionData {
    fn new() -> Self {
        Self { discriminator: 7 }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct CreateShardInstructionArgs {
    pub shard_number: u8,
}

fn create_create_shard_instruction(
    inscription_shard_account: Pubkey,
    payer: Pubkey,
    random_shard: u8,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(inscription_shard_account, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let mut data = CreateShardInstructionData::new().try_to_vec().unwrap();
    let mut args = CreateShardInstructionArgs {
        shard_number: random_shard,
    }
    .try_to_vec()
    .unwrap();

    data.append(&mut args);

    Instruction {
        program_id: INSCRIPTION_PROGRAM_ID,
        accounts,
        data,
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // // random between 0-31
    // let random_shard: u8 = rand::thread_rng().gen_range(0..32);
    // info!("random_shard: {}", random_shard);

    let inscription_shard_account = Pubkey::find_program_address(
        &[
            "Inscription".as_bytes(),
            "Shard".as_bytes(),
            INSCRIPTION_PROGRAM_ID.as_ref(),
            input.shard_number.to_le_bytes().as_ref(),
        ],
        &INSCRIPTION_PROGRAM_ID,
    )
    .0;

    info!("inscription_shard_account: {}", inscription_shard_account);

    // // fetch shard account and check if it exists
    // let shard_account = match ctx
    //     .solana_client()
    //     .get_account(&inscription_shard_account)
    //     .await
    // {
    //     Ok(shard_account) => Some(shard_account),
    //     Err(e) => {
    //         dbg!("shard account not found, creating");
    //         None
    //     }
    // };

    // let shard_account = InscriptionShard::try_from(
    //     &((inscription_shard_account, shard_account).into_account_info()),
    // )?;

    // if shard_account.key != Key::InscriptionShardAccount {
    //     // create shard account
    //     dbg!("creating shard account for {}", inscription_shard_account);
    // }

    let create_shard_instruction = create_create_shard_instruction(
        inscription_shard_account,
        input.fee_payer.pubkey(),
        input.shard_number,
    );

    info!("create_shard_instruction: {:?}", create_shard_instruction);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone()].into_iter().collect(),
        instructions: [create_shard_instruction].into(),
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
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
