use crate::{prelude::*, wormhole::WormholeInstructions};

use borsh::{BorshDeserialize, BorshSerialize};
use rand::Rng;
use solana_program::{instruction::AccountMeta, system_instruction, sysvar};
use solana_sdk::pubkey::Pubkey;

use super::{BridgeData, PostMessageData};

// Command Name
const NAME: &str = "post_message";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/wormhole/post_message.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub emitter: Keypair,
    #[serde(with = "value::keypair")]
    pub message: Keypair,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.cfg.solana_client.cluster);

    // TODO: use a real nonce
    let nonce = rand::thread_rng().gen();

    let emitter = input.emitter.pubkey();

    let bridge = Pubkey::find_program_address(&[b"Bridge"], &wormhole_core_program_id).0;

    let fee_collector =
        Pubkey::find_program_address(&[b"fee_collector"], &wormhole_core_program_id).0;

    let sequence =
        Pubkey::find_program_address(&[b"Sequence", emitter.as_ref()], &wormhole_core_program_id).0;

    // TODO test payload
    let _payload = [0u8; 32].to_vec();
    let payload = "Hello World!".as_bytes().to_vec();

    let ix = solana_program::instruction::Instruction {
        program_id: wormhole_core_program_id,
        accounts: vec![
            AccountMeta::new(bridge, false),
            AccountMeta::new(input.message.pubkey(), true),
            AccountMeta::new_readonly(emitter, true),
            AccountMeta::new(sequence, false),
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new(fee_collector, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: (
            WormholeInstructions::PostMessage,
            PostMessageData {
                nonce,
                payload: payload.to_vec(),
                consistency_level: super::ConsistencyLevel::Confirmed,
            },
        )
            .try_to_vec()?,
    };

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_bubblegum::accounts::CreateTree,
        >())
        .await?;

    // Get message fee
    let bridge_config_account = ctx.solana_client.get_account(&bridge).await?;
    let bridge_config = BridgeData::try_from_slice(bridge_config_account.data.as_slice())?;
    let fee = bridge_config.config.fee;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.emitter.clone_keypair(),
            input.message.clone_keypair(),
        ]
        .into(),
        instructions: [
            system_instruction::transfer(&input.payer.pubkey(), &fee_collector, fee),
            ix,
        ]
        .into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "sequence" => sequence,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
