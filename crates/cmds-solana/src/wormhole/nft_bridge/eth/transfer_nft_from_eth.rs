use crate::{prelude::*, wormhole::token_bridge::eth::Response as ServerlessOutput};

// Command Name
const NAME: &str = "transfer_nft_from_eth";

const DEFINITION: &str = include_str!(
    "../../../../../../node-definitions/solana/wormhole/nft_bridge/eth/transfer_nft_from_eth.json"
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
    pub token: String,
    pub network_name: String,
    pub recipient: String,
    pub token_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    response: ServerlessOutput,
    emitter: String,
    sequence: String,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        token: String,
        keypair: String,
        recipient: String,
        #[serde(rename = "tokenId")]
        token_id: String,
    }

    let payload = Payload {
        network_name: input.network_name,
        token: input.token,
        keypair: input.keypair,
        recipient: input.recipient,
        token_id: input.token_id,
    };

    let response: ServerlessOutput = ctx
        .http
        .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/transfer_nft_from_eth")
        .json(&payload)
        .send()
        .await?
        .json::<ServerlessOutput>()
        .await?;

    let emitter = response.output.emitter_address.clone();
    let sequence = response.output.sequence.clone();

    Ok(Output {
        response,
        emitter,
        sequence,
    })
}

#[cfg(test)]
mod tests {
    use crate::wormhole::token_bridge::eth::Receipt;

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        token: String,
        keypair: String,
        recipient: String,
        #[serde(rename = "tokenId")]
        token_id: String,
    }

    #[tokio::test]
    async fn need_key_test_local() {
        let _json_input = r#"{
            "output": {
                "receipt": {
                    "to": "0xD8E4C2DbDd2e2bd8F1336EA691dBFF6952B1a6eB",
                    "from": "0xdD6c5B9eA3Ac0FB5387E5e6B482788d5F70772A6",
                    "contractAddress": null,
                    "transactionIndex": 8,
                    "gasUsed": {
                        "type": "BigNumber",
                        "hex": "0x578c"
                    },
                    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "blockHash": "0x4eb1e80788dfed4d50a5bf72d5ece34f023e796ebb522d0102997cc8b066c49f",
                    "transactionHash": "0x0b911086660107e379011b76a5841626db0b67df80f4734ed12ddceef8f41799",
                    "logs": [],
                    "blockNumber": 4330148,
                    "confirmations": 1,
                    "cumulativeGasUsed": {
                        "type": "BigNumber",
                        "hex": "0x23ebec"
                    },
                    "effectiveGasPrice": {
                        "type": "BigNumber",
                        "hex": "0x59682f08"
                    },
                    "status": 1,
                    "type": 2,
                    "byzantium": true,
                    "events": []
                }
            }
        }"#;

        async fn test(payload: Payload) -> Result<Receipt, reqwest::Error> {
            let client = reqwest::Client::new();
            let response = client
                .post(
                    "https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/transfer_nft_from_eth",
                )
                .json(&payload)
                .send()
                .await?
                .json::<ServerlessOutput>()
                .await?;

            let receipt = response.output.receipt;

            Ok(receipt)
        }

        let payload = Payload {
            network_name: "devnet".into(),
            token: "0xDB5492265f6038831E89f495670FF909aDe94bd9".into(),
            keypair: "".into(),
            recipient: "0x00000000".into(),
            token_id: "0".into(),
        };

        let res = test(payload).await.unwrap();
        dbg!(res);
    }
}
