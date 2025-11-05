use super::{PostVAAData, VAA};
use crate::{prelude::*, wormhole::WormholeInstructions};
use solana_program::pubkey::Pubkey;
use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk_ids::system_program;

// Command Name
const NAME: &str = "post_vaa";

const DEFINITION: &str = flow_lib::node_definition!("wormhole/post_vaa.json");

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
    pub guardian_set_index: u32,
    pub vaa_hash: bytes::Bytes,
    pub vaa: bytes::Bytes,
    // TODO: not in signers list
    pub signature_set: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.solana_config().cluster);
    let bridge = Pubkey::find_program_address(&[b"Bridge"], &wormhole_core_program_id).0;

    let guardian_set = Pubkey::find_program_address(
        &[b"GuardianSet", &input.guardian_set_index.to_le_bytes()],
        &wormhole_core_program_id,
    )
    .0;

    let vaa_address =
        Pubkey::find_program_address(&[b"PostedVAA", &input.vaa_hash], &wormhole_core_program_id).0;

    let vaa =
        VAA::deserialize(&input.vaa).map_err(|_| anyhow::anyhow!("Failed to deserialize VAA"))?;

    let vaa: PostVAAData = vaa.into();

    let ix = solana_program::instruction::Instruction {
        program_id: wormhole_core_program_id,
        accounts: vec![
            AccountMeta::new_readonly(guardian_set, false),
            AccountMeta::new_readonly(bridge, false),
            AccountMeta::new_readonly(input.signature_set.pubkey(), false),
            AccountMeta::new(vaa_address, false),
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: borsh::to_vec(&(WormholeInstructions::PostVAA, vaa))?,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer].into(),
        instructions: [ix].into(),
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
                "vaa_address" => vaa_address,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
