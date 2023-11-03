use crate::prelude::*;
use base64::decode;
use primitive_types::U256;
use wormhole_sdk::{nft::Message as NftMessage, vaa::Digest, Address, Chain, Vaa};

use super::MessageAlias;

// Command Name
const NAME: &str = "parse_vaa";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/wormhole/parse_vaa.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: Lazy<Result<CmdBuilder, BuilderError>> =
        Lazy::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub vaa: String,
    // pub vaa_payload_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    parsed_vaa: Vaa<Vec<u8>>,
    vaa_bytes: bytes::Bytes,
    signatures: Vec<wormhole_sdk::vaa::Signature>,
    body: bytes::Bytes,
    vaa_hash: bytes::Bytes,
    vaa_secp256k_hash: bytes::Bytes,
    guardian_set_index: u32,
    payload: serde_json::Value,
    nft_token_id: Option<String>,
}

async fn run(_ctx: Context, input: Input) -> Result<Output, CommandError> {
    let vaa_string = &input.vaa;

    let vaa_bytes = decode(vaa_string)
        .map_err(|err| anyhow::anyhow!("Failed to decode VAA string: {}", err))?;

    let sig_start = 6;
    let num_signers = vaa_bytes[5] as usize;
    let sig_length = 66;

    let mut guardian_signatures = Vec::new();
    for i in 0..num_signers {
        let start = sig_start + i * sig_length;
        let mut signature = [0u8; 65];
        signature.copy_from_slice(&vaa_bytes[start + 1..start + 66]);
        guardian_signatures.push(wormhole_sdk::vaa::Signature {
            index: vaa_bytes[start],
            signature,
        });
    }

    let body = &vaa_bytes[sig_start + sig_length * num_signers..];
    // Check this https://github.com/wormhole-foundation/wormhole/blob/14a1251c06b3d837dcbd2b7bed5b1abae6eb7d02/solana/bridge/program/src/vaa.rs#L176
    let parsed_vaa: Vaa<Vec<u8>> = Vaa {
        version: vaa_bytes[0],
        guardian_set_index: u32::from_be_bytes(
            vaa_bytes[1..5]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert guardian_set_index"))?,
        ),
        signatures: guardian_signatures.clone(),
        timestamp: u32::from_be_bytes(
            body[0..4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert timestamp"))?,
        ),
        nonce: u32::from_be_bytes(
            body[4..8]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert nonce"))?,
        ),
        emitter_chain: Chain::from(u16::from_be_bytes(
            body[8..10]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert emitter_chain"))?,
        )),
        emitter_address: Address(
            body[10..42]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert emitter_address"))?,
        ),
        sequence: u64::from_be_bytes(
            body[42..50]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert sequence"))?,
        ),
        consistency_level: body[50],
        // gets converted to base64 string?
        payload: body[51..].to_vec(),
    };

    // let (_, body): (Header, Body<Vec<u8>>) = parsed_vaa.into();

    let Digest {
        hash: vaa_hash,
        secp256k_hash: vaa_secp256k_hash,
    } = wormhole_sdk::vaa::digest(body).map_err(|_| anyhow::anyhow!("Failed to digest VAA"))?;

    let payload = match serde_wormhole::from_slice(&parsed_vaa.payload) {
        Ok(message) => MessageAlias::Transfer(message),
        Err(_) => match serde_wormhole::from_slice(&parsed_vaa.payload) {
            Ok(nft_message) => MessageAlias::NftTransfer(nft_message),
            Err(_) => return Err(anyhow::anyhow!("Payload content not supported")),
        },
    };

    let output_payload: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&payload)?)?;

    let output_payload = output_payload
        .get("NftTransfer")
        .or(output_payload.get("Transfer"))
        .ok_or_else(|| anyhow::anyhow!("Invalid payload"))?;

    let token_id = match &payload {
        MessageAlias::NftTransfer(message) => match message {
            NftMessage::Transfer {
                token_id,
                nft_address: _,
                nft_chain: _,
                symbol: _,
                name: _,
                uri: _,
                to: _,
                to_chain: _,
            } => Some(token_id),
        },
        _ => None,
    };

    let nft_token_id = token_id.map(|token_id| U256::from_big_endian(&token_id.0).to_string());

    Ok(Output {
        parsed_vaa: parsed_vaa.clone(),
        vaa_bytes: bytes::Bytes::copy_from_slice(&vaa_bytes),
        signatures: guardian_signatures,
        body: bytes::Bytes::copy_from_slice(body),
        vaa_hash: bytes::Bytes::copy_from_slice(&vaa_hash),
        vaa_secp256k_hash: bytes::Bytes::copy_from_slice(&vaa_secp256k_hash),
        guardian_set_index: parsed_vaa.guardian_set_index,
        payload: output_payload.clone(),
        nft_token_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wormhole_sdk::token::Message;

    #[test]
    fn test() -> Result<(), anyhow::Error> {
        //sol vaa, not supported payload
        let _vaa_string:String = "AQAAAAABAE9eT/T0B917C5+ZQEHdlDUD/b7PNfTkyy/mXX7LPSJzVS6VTJx1gigK7xCic3UywM5/ehtUnZ/HCdoLQtOLX1IBZLYUVg1YsBoAAcARZHHBCI3jyzPKm9l0vBFJ3DJ4Yh+vmP6ZmTrfVHxrAAAAAAAAAAABSGVsbG8gV29ybGQh".to_string();
        let vaa_string:String ="AQAAAAABAMy+FBjMJafK1Xt4cCSbJ03jxJs3f3UW647HrdpT34XWE/7CBbQjo+0xMQXDTlh5IymI6wissEo8TkxTwY/ufCwBZMMBLO/WHgoAATsmQJ+Kre0/XdyhhGlapqD6gpsMhcr4SFYySJbSFMqYAAAAAAAAX3cgAv+98jdq256Gu41IuSzwRBryKQ5Ku3e8LsfhFUYdQ2pkAAEJAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==".to_string();
        // //eth vaa
        // let vaa_string:String="AQAAAAABAHZle4NbI4+ItAFCCwtKYDthhzq61u1az/gZIbW+hQ8MRskKSDEvutVy7pjuRwRq7EsKhB/lMz4XDDxoeyVm6YkBZMASCPZ6AAAnEgAAAAAAAAAAAAAAANtUkiZfYDiDHon0lWcP+Qmt6UvZAAAAAAAAAZgBAgAAAAAAAAAAAAAAAEEKixUC8B8oh/CwWyLMk01FpiinJxISRVJDX1NZTUJPTAAAAAAAAAAAAAAAAAAAAAAAAAAAAABNeUVSQzIwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==".to_string();

        // //eth transfer vaa
        // let vaa_string:String="AQAAAAABAIDirkZb0u0i33P55FM8+ErUor6LbHELePcpfMyC3JRHPFQJ7ztwLOI9XlwvK1cqgSQC8Q+4hh/gyV5W8/rKt2cBZMFSePdTAQAnEgAAAAAAAAAAAAAAANtUkiZfYDiDHon0lWcP+Qmt6UvZAAAAAAAAAacBAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAF9eEAAAAAAAAAAAAAAAAAQQqLFQLwHyiH8LBbIsyTTUWmKKcnEi26xJVia/fd3KTtEQn+ZwcAonBDCzA1vRw+oHhAWKEJAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==".to_string();

        // NFT vaa from ETH
        // let vaa_string ="AQAAAAABAK2WRJ3P2kYYzQTI1P9QS7PK19hPBic2XYQviPBIzGqOISa+/M6aSwm/2VyKfVEPvAfDXbhKqpOpeHuifzlSluwBZQ3JzfuW1JYAAXUqSYFOQLlrCXIH5LU/3TMFROHmYWU/utS8FZzCioOeAAAAAAAAAKsgAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAAFTUE9QAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFNPICMxMTExMQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA0YcsTJvXJtMAUWOeXRBneS7i9QqAN/hIBs4rqasbOrnIaHR0cHM6Ly9hcndlYXZlLm5ldC8zRnhwSUlicHlTbmZUVFhJcnBvamhGMktISGpldkk4TXJ0M3BBQ21FYlNZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADdbFueo6wPtTh+XmtIJ4jV9wdypicS".to_string();

        let vaa_bytes = decode(vaa_string).unwrap();

        let sig_start = 6;
        let num_signers = vaa_bytes[5] as usize;
        let sig_length = 66;

        let mut guardian_signatures = Vec::new();
        for i in 0..num_signers {
            let start = sig_start + i * sig_length;
            let mut signature = [0u8; 65];
            signature.copy_from_slice(&vaa_bytes[start + 1..start + 66]);
            guardian_signatures.push(wormhole_sdk::vaa::Signature {
                index: vaa_bytes[start],
                signature,
            });
        }

        let body = &vaa_bytes[sig_start + sig_length * num_signers..];

        let parsed_vaa = Vaa {
            version: vaa_bytes[0],
            guardian_set_index: u32::from_be_bytes(vaa_bytes[1..5].try_into().unwrap()),
            signatures: guardian_signatures,
            timestamp: u32::from_be_bytes(body[0..4].try_into().unwrap()),
            nonce: u32::from_be_bytes(body[4..8].try_into().unwrap()),
            emitter_chain: Chain::from(u16::from_be_bytes(body[8..10].try_into().unwrap())),
            emitter_address: Address(body[10..42].try_into().unwrap()),
            sequence: u64::from_be_bytes(body[42..50].try_into().unwrap()),
            consistency_level: body[50],
            payload: body[51..].to_vec(),
        };

        #[derive(Serialize, Deserialize, Debug)]
        enum MessageAlias {
            Transfer(Message),
            NftTransfer(NftMessage),
        }

        let payload = match serde_wormhole::from_slice(&parsed_vaa.payload) {
            Ok(message) => MessageAlias::Transfer(message),
            Err(_) => match serde_wormhole::from_slice(&parsed_vaa.payload) {
                Ok(nft_message) => MessageAlias::NftTransfer(nft_message),
                Err(_) => return Err(anyhow::anyhow!("Payload content not supported")),
            },
        };

        let token_id = match &payload {
            MessageAlias::NftTransfer(message) => match message {
                NftMessage::Transfer {
                    token_id,
                    nft_address: _,
                    nft_chain: _,
                    symbol: _,
                    name: _,
                    uri: _,
                    to: _,
                    to_chain: _,
                } => Some(token_id),
            },
            _ => None,
        };

        let _token_id = token_id.map(|token_id| U256::from_big_endian(&token_id.0).to_string());
        // dbg!(token_id);
        // panic!("test");

        // Convert token id

        // let payload_value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&payload)?)?;

        // let inner_json = payload_value
        //     .get("NftTransfer")
        //     .or(payload_value.get("Transfer"))
        //     .ok_or_else(|| anyhow::anyhow!("Invalid payload"))?;

        // dbg!(&parsed_vaa);
        // dbg!(&inner_json.to_string());

        // let string = String::from_utf8(parsed_vaa.payload).unwrap();
        // println!("{}", string);
        // dbg!(&vaa_bytes);

        // let token_id_str = inner_json["1"]["token_id"].as_str().ok_or_else(|| anyhow::anyhow!("Token ID not found"))?;
        // let token_id = U256::from_dec_str(token_id_str).map_err(|_| anyhow::anyhow!("Invalid token ID"))?;
        // let mut token_id_bytes = vec![0u8; 32];
        // token_id.to_big_endian(&mut token_id_bytes);
        // dbg!(token_id);

        // let token_id_bytes = U256::from_dec_str(token_id)
        //     .map_err(|_| anyhow::anyhow!("Invalid token ID"))?
        //     .to_big_endian();
        // dbg!(token_id_bytes);

        // let token_id_input =
        //     U256::from_str(token_id_str).map_err(|_| anyhow::anyhow!("Invalid token id"))?;
        // let mut token_id = vec![0u8; 32];
        // token_id_input.to_big_endian(&mut token_id);
        Ok(())
    }
}
