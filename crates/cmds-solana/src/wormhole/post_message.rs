use crate::{prelude::*, wormhole::WormholeInstructions};

use borsh::BorshDeserialize;
use rand::Rng;
use solana_program::pubkey::Pubkey;
use solana_program::{instruction::AccountMeta, sysvar};
use solana_system_interface::instruction::transfer;

use super::{BridgeData, PostMessageData, token_bridge::get_sequence_number_from_message};

// Command Name
const NAME: &str = "post_message";

const DEFINITION: &str = flow_lib::node_definition!("wormhole/post_message.json");

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
    pub payer: Wallet,
    pub emitter: Wallet,
    pub message: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    sequence: String,
    emitter: String,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.solana_config().cluster);

    // TODO: use a real nonce
    let nonce = rand::thread_rng().r#gen();

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
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data: borsh::to_vec(&(
            WormholeInstructions::PostMessage,
            PostMessageData {
                nonce,
                payload: payload.to_vec(),
                consistency_level: super::ConsistencyLevel::Confirmed,
            },
        ))?,
    };

    // Get message fee
    let bridge_config_account = ctx.solana_client().get_account(&bridge).await?;
    let bridge_config = BridgeData::try_from_slice(bridge_config_account.data.as_slice())?;
    let fee = bridge_config.config.fee;

    let message_pubkey = input.message.pubkey();

    let instructions = [transfer(&input.payer.pubkey(), &fee_collector, fee), ix].into();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.emitter, input.message].into(),
        instructions,
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    let sequence = get_sequence_number_from_message(&ctx, message_pubkey).await?;

    Ok(Output {
        signature,
        sequence,
        emitter: emitter.to_string(),
    })
}
