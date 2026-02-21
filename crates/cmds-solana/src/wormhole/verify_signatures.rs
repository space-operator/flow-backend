use super::{GuardianSetData, SignatureItem, VerifySignaturesData};
use crate::{prelude::*, wormhole::WormholeInstructions};
use borsh::BorshDeserialize;
use byteorder::{LittleEndian, WriteBytesExt};
use solana_program::pubkey::Pubkey;
use solana_program::{instruction::AccountMeta, sysvar};
use std::io::Write;

// Command Name
const NAME: &str = "verify_signatures";

const DEFINITION: &str = flow_lib::node_definition!("wormhole/verify_signatures.jsonc");

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
    pub signature_set: Wallet,
    pub guardian_set_index: u32,
    pub signatures: Vec<wormhole_sdk::vaa::Signature>,
    pub vaa_body: bytes::Bytes,
    pub vaa_hash: bytes::Bytes,
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

    let guardian_set = Pubkey::find_program_address(
        &[b"GuardianSet", &input.guardian_set_index.to_le_bytes()],
        &wormhole_core_program_id,
    )
    .0;

    let account: solana_account::Account = ctx
        .solana_client()
        .get_account(&guardian_set)
        .await
        .unwrap();
    let guardian_set_data: GuardianSetData =
        GuardianSetData::try_from_slice(&account.data).unwrap();

    let mut signature_items: Vec<SignatureItem> = Vec::new();
    for s in input.signatures.iter() {
        let mut item = SignatureItem {
            signature: s.signature.to_vec(),
            key: [0; 20],
            index: s.index,
        };
        item.key = guardian_set_data.keys[s.index as usize];

        signature_items.push(item);
    }

    let mut verify_txs: Vec<Vec<Instruction>> = Vec::new();

    for chunk in signature_items.chunks(7) {
        let mut secp_payload = Vec::new();
        let mut signature_status = [-1i8; 19];

        let data_offset = 1 + chunk.len() * 11;
        let message_offset = data_offset + chunk.len() * 85;

        // 1 number of signatures
        secp_payload.write_u8(chunk.len() as u8)?;

        // Secp signature info description (11 bytes * n)
        for (i, s) in chunk.iter().enumerate() {
            secp_payload.write_u16::<LittleEndian>((data_offset + 85 * i) as u16)?;
            secp_payload.write_u8(0)?;
            secp_payload.write_u16::<LittleEndian>((data_offset + 85 * i + 65) as u16)?;
            secp_payload.write_u8(0)?;
            secp_payload.write_u16::<LittleEndian>(message_offset as u16)?;
            secp_payload.write_u16::<LittleEndian>(input.vaa_hash.len() as u16)?;
            secp_payload.write_u8(0)?;
            signature_status[s.index as usize] = i as i8;
        }

        // Write signatures and addresses
        for s in chunk.iter() {
            secp_payload.write_all(&s.signature)?;
            secp_payload.write_all(&s.key)?;
        }

        // Write body
        secp_payload.write_all(&input.vaa_hash)?;

        let secp_ix = Instruction {
            program_id: solana_program::secp256k1_program::id(),
            data: secp_payload,
            accounts: vec![],
        };

        let payload = VerifySignaturesData {
            signers: signature_status,
        };

        let verify_ix = Instruction {
            program_id: wormhole_core_program_id,
            accounts: vec![
                AccountMeta::new(input.payer.pubkey(), true),
                AccountMeta::new_readonly(guardian_set, false),
                AccountMeta::new(input.signature_set.pubkey(), true),
                AccountMeta::new_readonly(sysvar::instructions::id(), false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ],

            data: borsh::to_vec(&(WormholeInstructions::VerifySignatures, payload))?,
        };

        verify_txs.push(vec![secp_ix, verify_ix])
    }

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.signature_set].into(),
        instructions: verify_txs.concat(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
