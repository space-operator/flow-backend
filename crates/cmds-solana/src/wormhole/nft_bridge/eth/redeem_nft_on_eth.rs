use crate::{
    prelude::*,
    wormhole::token_bridge::eth::{Receipt, RedeemOnEthResponse},
};

// Command Name
const NAME: &str = "redeem_nft_on_eth";

const DEFINITION: &str = include_str!(
    "../../../../../../node-definitions/solana/wormhole/nft_bridge/eth/redeem_nft_on_eth.json"
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    receipt: Receipt,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        keypair: String,
        #[serde(rename = "signedVAA")]
        signed_vaa: String,
    }

    let payload = Payload {
        network_name: input.network_name,
        keypair: input.keypair,
        signed_vaa: input.signed_vaa,
    };

    let response: RedeemOnEthResponse = ctx
        .http
        .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/redeem_nft_on_eth")
        .json(&payload)
        .send()
        .await?
        .json::<RedeemOnEthResponse>()
        .await?;

    let receipt: Receipt = response.output.receipt;

    // to is the wormhole token bridge contract
    // from is the recipient
    // logs/address is the transferred token contract address

    Ok(Output { receipt })
}

#[cfg(test)]
mod tests {
    use crate::wormhole::token_bridge::eth::RedeemOnEthResponse;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        keypair: String,
        #[serde(rename = "signedVAA")]
        signed_vaa: String,
    }

    #[tokio::test]
    async fn need_key_test_local() {
        let _json_response = r#"{
            "output": Object {
                "receipt": Object {
                    "to": String("0xDB5492265f6038831E89f495670FF909aDe94bd9"),
                    "from": String("0xdD6c5B9eA3Ac0FB5387E5e6B482788d5F70772A6"),
                    "contractAddress": Null,
                    "transactionIndex": Number(24),
                    "gasUsed": Object {
                        "type": String("BigNumber"),
                        "hex": String("0x02916e"),
                    },
                    "logsBloom": String("0x00000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000040000008000400000000000000000000000000000000000000000000020000000000000000000800000000000002000000000010000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000"),
                    "blockHash": String("0x910302d187cea8989abf11f08994b49508b6e9bec8f15c1e837370af722c70c0"),
                    "transactionHash": String("0xc3da8759b01f0f04ff0d0aad5594d69888bd5d2cde0e0236248fcdb50b51dcab"),
                    "logs": Array [
                        Object {
                            "transactionIndex": Number(24),
                            "blockNumber": Number(4205532),
                            "transactionHash": String("0xc3da8759b01f0f04ff0d0aad5594d69888bd5d2cde0e0236248fcdb50b51dcab"),
                            "address": String("0x44C80265b027b4Fed63C177f3Ed9C174a0f417d1"),
                            "topics": Array [
                                String("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"),
                                String("0x0000000000000000000000000000000000000000000000000000000000000000"),
                                String("0x000000000000000000000000dd6c5b9ea3ac0fb5387e5e6b482788d5f70772a6"),
                            ],
                            "data": String("0x00000000000000000000000000000000000000000000000000000002540be400"),
                            "logIndex": Number(45),
                            "blockHash": String("0x910302d187cea8989abf11f08994b49508b6e9bec8f15c1e837370af722c70c0"),
                        },
                    ],
                    "blockNumber": Number(4205532),
                    "confirmations": Number(1),
                    "cumulativeGasUsed": Object {
                        "type": String("BigNumber"),
                        "hex": String("0x763888"),
                    },
                    "effectiveGasPrice": Object {
                        "type": String("BigNumber"),
                        "hex": String("0x68f8dff3"),
                    },
                    "status": Number(1),
                    "type": Number(2),
                    "byzantium": Bool(true),
                    "events": Array [
                        Object {
                            "transactionIndex": Number(24),
                            "blockNumber": Number(4205532),
                            "transactionHash": String("0xc3da8759b01f0f04ff0d0aad5594d69888bd5d2cde0e0236248fcdb50b51dcab"),
                            "address": String("0x44C80265b027b4Fed63C177f3Ed9C174a0f417d1"),
                            "topics": Array [
                                String("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"),
                                String("0x0000000000000000000000000000000000000000000000000000000000000000"),
                                String("0x000000000000000000000000dd6c5b9ea3ac0fb5387e5e6b482788d5f70772a6"),
                            ],
                            "data": String("0x00000000000000000000000000000000000000000000000000000002540be400"),
                            "logIndex": Number(45),
                            "blockHash": String("0x910302d187cea8989abf11f08994b49508b6e9bec8f15c1e837370af722c70c0"),
                        },
                    ],
                },
            },
        }"#;

        async fn test(payload: Payload) -> Result<RedeemOnEthResponse, reqwest::Error> {
            let client = reqwest::Client::new();
            let json = client
                .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/redeem_nft_on_eth")
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
            keypair: "0x1bb0ed141673d3228d6dc10806f0de5ee6522695160aed8fb99e487a9abc622c".into(),
            signed_vaa: "AQAAAAABANOioLxunWtMG55i8Sbn9l2UMNrf50Vh9XDEb1vn9ZhRY5KuhjiXRVeM4aZ/xCUR+Oem5bZRRLZBIRg+Xa6WcTsAZQ0MVNHVy/YAAXUqSYFOQLlrCXIH5LU/3TMFROHmYWU/utS8FZzCioOeAAAAAAAAAKUgAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAAFTUE9QAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFNPICMxMTExMQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA4q3NjFNInmCE+RA3bMIlNx5NQRkCcSdoXMxHOFc/8wjIaHR0cHM6Ly9hcndlYXZlLm5ldC8zRnhwSUlicHlTbmZUVFhJcnBvamhGMktISGpldkk4TXJ0M3BBQ21FYlNZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADdbFueo6wPtTh+XmtIJ4jV9wdypicm".into(),
        };

        let res = test(payload).await.unwrap();
        dbg!(res);
    }
}
