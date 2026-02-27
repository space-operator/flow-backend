use crate::prelude::*;
use crate::wormhole::nft_bridge::Address;
use crate::wormhole::{PostVAAData, VAA};
use solana_program::pubkey::Pubkey;
use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk_ids::system_program;
use tracing::info;
use wormhole_sdk::nft::Message;

use super::{CompleteWrappedData, NFTBridgeInstructions, PayloadTransfer};

// Command Name
const NAME: &str = "nft_complete_wrapped";

const DEFINITION: &str =
    flow_lib::node_definition!("wormhole/nft_bridge/nft_complete_wrapped.jsonc");

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
    pub vaa: bytes::Bytes,
    // pub vaa: String,
    pub payload: wormhole_sdk::nft::Message,
    // pub payload: serde_json::Value,
    pub vaa_hash: bytes::Bytes,
    #[serde(with = "value::pubkey")]
    pub to_authority: Pubkey,
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

    let nft_bridge_program_id = crate::wormhole::nft_bridge_program_id(ctx.solana_config().cluster);

    let config_key = Pubkey::find_program_address(&[b"config"], &nft_bridge_program_id).0;

    let vaa =
        VAA::deserialize(&input.vaa).map_err(|_| anyhow::anyhow!("Failed to deserialize VAA"))?;

    let vaa: PostVAAData = vaa.into();

    let payload: PayloadTransfer = match input.payload {
        Message::Transfer {
            nft_address,
            nft_chain,
            symbol,
            name,
            token_id,
            uri,
            to,
            to_chain,
        } => PayloadTransfer {
            token_address: nft_address.0,
            token_chain: nft_chain.into(),
            to: Address(to.0),
            to_chain: to_chain.into(),
            symbol: symbol.to_string(),
            name: name.to_string(),
            token_id: primitive_types::U256::from_big_endian(&token_id.0),
            uri: uri.to_string(),
        },
    };

    // Convert token id
    let mut token_id = vec![0u8; 32];
    payload.token_id.to_big_endian(&mut token_id);

    let message =
        Pubkey::find_program_address(&[b"PostedVAA", &input.vaa_hash], &wormhole_core_program_id).0;

    let claim_key = Pubkey::find_program_address(
        &[
            vaa.emitter_address.as_ref(),
            vaa.emitter_chain.to_be_bytes().as_ref(),
            vaa.sequence.to_be_bytes().as_ref(),
        ],
        &nft_bridge_program_id,
    )
    .0;

    let endpoint = Pubkey::find_program_address(
        &[
            vaa.emitter_chain.to_be_bytes().as_ref(),
            vaa.emitter_address.as_ref(),
        ],
        &nft_bridge_program_id,
    )
    .0;

    let mint = Pubkey::find_program_address(
        &[
            b"wrapped",
            payload.token_chain.to_be_bytes().as_ref(),
            payload.token_address.as_ref(),
            token_id.as_ref(),
        ],
        &nft_bridge_program_id,
    )
    .0;
    info!("mint: {:?}", mint);

    let mint_meta =
        Pubkey::find_program_address(&[b"meta", mint.as_ref()], &nft_bridge_program_id).0;

    let mint_authority = Pubkey::find_program_address(&[b"mint_signer"], &nft_bridge_program_id).0;

    let to = Pubkey::from(payload.to.0);
    // let to = spl_associated_token_account_interface::address::get_associated_token_address(&input.to_authority, &mint);
    info!("to: {:?}", to);

    let ix = solana_program::instruction::Instruction {
        program_id: nft_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(message, false),
            AccountMeta::new(claim_key, false),
            AccountMeta::new_readonly(endpoint, false),
            AccountMeta::new(to, false),
            AccountMeta::new_readonly(input.to_authority, false),
            AccountMeta::new(mint, false),
            AccountMeta::new(mint_meta, false),
            AccountMeta::new_readonly(mint_authority, false),
            // Dependencies
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            // Program
            AccountMeta::new_readonly(wormhole_core_program_id, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false),
            AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        ],
        data: borsh::to_vec(&(
            NFTBridgeInstructions::CompleteWrapped,
            CompleteWrappedData {},
        ))?,
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
                "mint_metadata" => mint_meta,
                "mint" => mint,
                "token_account"=> to
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

// #[cfg(test)]
// mod tests {
//     use crate::wormhole::token_bridge::eth::Receipt;

//     use super::*;

//     #[derive(Serialize, Deserialize, Debug)]
//     struct Payload {
//         #[serde(rename = "networkName")]
//         network_name: String,
//         token: String,
//         keypair: String,
//         recipient: String,
//         #[serde(rename = "tokenId")]
//         token_id: String,
//     }

//     #[tokio::test]
//     async fn need_key_test_local() {
//         let _json_input = r#"{
//             "output": {
//                 "receipt": {
//                     "to": "0xD8E4C2DbDd2e2bd8F1336EA691dBFF6952B1a6eB",
//                     "from": "0xdD6c5B9eA3Ac0FB5387E5e6B482788d5F70772A6",
//                     "contractAddress": null,
//                     "transactionIndex": 8,
//                     "gasUsed": {
//                         "type": "BigNumber",
//                         "hex": "0x578c"
//                     },
//                     "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
//                     "blockHash": "0x4eb1e80788dfed4d50a5bf72d5ece34f023e796ebb522d0102997cc8b066c49f",
//                     "transactionHash": "0x0b911086660107e379011b76a5841626db0b67df80f4734ed12ddceef8f41799",
//                     "logs": [],
//                     "blockNumber": 4330148,
//                     "confirmations": 1,
//                     "cumulativeGasUsed": {
//                         "type": "BigNumber",
//                         "hex": "0x23ebec"
//                     },
//                     "effectiveGasPrice": {
//                         "type": "BigNumber",
//                         "hex": "0x59682f08"
//                     },
//                     "status": 1,
//                     "type": 2,
//                     "byzantium": true,
//                     "events": []
//                 }
//             }
//         }"#;

//         async fn test(payload: Payload) -> Result<Receipt, reqwest::Error> {
//             let client = reqwest::Client::new();
//             let response = client
//                 .post(
//                     "https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/transfer_nft_from_eth",
//                 )
//                 .json(&payload)
//                 .send()
//                 .await?
//                 .json::<ServerlessOutput>()
//                 .await?;

//             let receipt = response.output.receipt;

//             Ok(receipt)
//         }

//         let payload = Payload {
//             network_name: "devnet".into(),
//             token: "0xDB5492265f6038831E89f495670FF909aDe94bd9".into(),
//             keypair: "".into(),
//             recipient: "0x00000000".into(),
//             token_id: "0".into(),
//         };

//         let res = test(payload).await.unwrap();
//         dbg!(res);
//     }
// }
