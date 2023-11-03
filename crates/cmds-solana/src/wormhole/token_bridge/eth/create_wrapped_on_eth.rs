use crate::{
    prelude::*,
    wormhole::token_bridge::{
        eth::{hex_to_address, CreateWrappedResponse},
        Address,
    },
};

use super::Receipt;

// Command Name
const NAME: &str = "create_wrapped_on_eth";

const DEFINITION: &str = include_str!(
    "../../../../../../node-definitions/solana/wormhole/token_bridge/eth/create_wrapped_on_eth.json"
);

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: Lazy<Result<CmdBuilder, BuilderError>> =
        Lazy::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));

    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub keypair: String,
    pub network_name: String,
    pub signed_vaa: String,
    #[serde(with = "value::pubkey")]
    pub token: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    receipt: Receipt,
    address: Address,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        keypair: String,
        #[serde(rename = "signedVAA")]
        signed_vaa: String,
        token: String,
    }

    let payload = Payload {
        network_name: input.network_name,
        keypair: input.keypair,
        signed_vaa: input.signed_vaa,
        token: input.token.to_string(),
    };

    let response: CreateWrappedResponse = ctx
        .http
        .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/create_wrapped_on_eth")
        .json(&payload)
        .send()
        .await?
        .json::<CreateWrappedResponse>()
        .await?;

    let receipt = response.output.receipt;

    // token contract address on ETH
    let address = hex_to_address(&response.output.address).map_err(anyhow::Error::msg)?;

    Ok(Output { receipt, address })
}

#[cfg(test)]
mod tests {
    use crate::wormhole::token_bridge::eth::CreateWrappedResponse;
    use serde::{Deserialize, Serialize};
    use std::{fmt::Write, num::ParseIntError};
    use wormhole_sdk::Address;

    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        keypair: String,
        #[serde(rename = "signedVAA")]
        signed_vaa: String,
        token: String,
    }

    #[tokio::test]
    async fn need_key_test_local() {
        async fn test(payload: Payload) -> Result<CreateWrappedResponse, reqwest::Error> {
            let client = reqwest::Client::new();
            let json = client
                .post(
                    "https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/create_wrapped_on_eth",
                )
                .json(&payload)
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            dbg!(&json);

            let response = serde_json::from_value(json).unwrap();

            Ok(response)
        }

        let payload = Payload {
            network_name: "devnet".into(),
            keypair: "".into(),
            signed_vaa: "AQAAAAABAG9er/MmJMZA+TXKhvruR6h07pgDs4jvGEX32tA/X+fPJoLN5GdryI2AnnKLeN/y2DG1XVfqQIjSwVmJrdFQ1JUAZNvkF/RT7/MAATsmQJ+Kre0/XdyhhGlapqD6gpsMhcr4SFYySJbSFMqYAAAAAAAAYqYgAmc+E+tQG8MVnhfmdvaOmyILEFx3DYlI+fuqLuFPMiDtAAEJAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==".into(),
       token:"7x1tu6xjxhCNnnwTNytmGYL6w4Cwe3PDMo7gmfc89GHa".into()
        };

        let res = test(payload).await.unwrap();
        dbg!(res);
    }

    pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }

    #[test]
    fn hex_to_address() -> Result<(), anyhow::Error> {
        let address = "0xc15B6515aC32a91ACe0b8fABEBBB924a6CD4A539";

        if !address.starts_with("0x") {
            return Err(anyhow::anyhow!("invalid address {}", address));
        };

        let stripped_address = address.split_at(2).1;

        let bytes = decode_hex(stripped_address).unwrap();
        let mut array = [0u8; 32];
        array[32 - bytes.len()..].copy_from_slice(&bytes);
        let address: Address = Address(array);
        dbg!(address.to_string());

        // back to string
        // remove left zero padding
        let mut s = String::new();
        s.push_str("0x");
        for b in address.0.iter() {
            if *b == 0 {
                continue;
            }
            write!(&mut s, "{:02x}", b).unwrap();
        }
        dbg!(s);
        Ok(())
    }
}
