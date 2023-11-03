use super::{GuardianSetData, SignatureItem, VerifySignaturesData};
use crate::{prelude::*, wormhole::WormholeInstructions};
use borsh::{BorshDeserialize, BorshSerialize};
use byteorder::{LittleEndian, WriteBytesExt};
use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk::pubkey::Pubkey;
use std::io::Write;

// Command Name
const NAME: &str = "verify_signatures";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/wormhole/verify_signatures.json");

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
    pub signature_set: Keypair,
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

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.cfg.solana_client.cluster);

    let guardian_set = Pubkey::find_program_address(
        &[b"GuardianSet", &input.guardian_set_index.to_le_bytes()],
        &wormhole_core_program_id,
    )
    .0;

    let account: solana_sdk::account::Account =
        ctx.solana_client.get_account(&guardian_set).await.unwrap();
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

    for (_tx_index, chunk) in signature_items.chunks(7).enumerate() {
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
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
            ],

            data: (WormholeInstructions::VerifySignatures, payload).try_to_vec()?,
        };

        verify_txs.push(vec![secp_ix, verify_ix])
    }

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_bubblegum::accounts::CreateTree,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.signature_set.clone_keypair(),
        ]
        .into(),
        instructions: verify_txs.concat(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
