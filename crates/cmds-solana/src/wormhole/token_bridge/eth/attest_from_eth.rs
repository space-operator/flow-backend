use crate::{prelude::*, wormhole::token_bridge::eth::Response as ServerlessOutput};

// Command Name
const NAME: &str = "attest_from_eth";

const DEFINITION: &str = include_str!(
    "../../../../../../node-definitions/solana/wormhole/token_bridge/eth/attest_from_eth.json"
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
    }

    let payload = Payload {
        network_name: input.network_name,
        token: input.token,
        keypair: input.keypair,
    };

    let response: ServerlessOutput = ctx
        .http
        .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/attest_from_eth")
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
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network_name: String,
        token: String,
        keypair: String,
    }

    #[tokio::test]
    async fn need_key_test_local() {
        let _json_input = r#"{
            "output": {
                "receipt": {
                    "to": "0xDB5492265f6038831E89f495670FF909aDe94bd9",
                    "from": "0xdD6c5B9eA3Ac0FB5387E5e6B482788d5F70772A6",
                    "contractAddress": null,
                    "transactionIndex": 20,
                    "gasUsed": {
                        "type": "BigNumber",
                        "hex": "0x010a74"
                    },
                    "logsBloom": "0x00000000000100000000000000000010000000000000000000000000000000010010000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "blockHash": "0x974ba2485dbb37cf9253cec90bab96f96b806b1ee1db8f8d4833f69258f635d3",
                    "transactionHash": "0x62f2b7d16c483b3ec76962fb5337b6a442458af0027941e717770afbb3769b08",
                    "logs": [
                        {
                            "transactionIndex": 20,
                            "blockNumber": 3957578,
                            "transactionHash": "0x62f2b7d16c483b3ec76962fb5337b6a442458af0027941e717770afbb3769b08",
                            "address": "0x4a8bc80Ed5a4067f1CCf107057b8270E0cC11A78",
                            "topics": [
                                "0x6eb224fb001ed210e379b335e35efe88672a8ce935d981a6896b27ffdf52a3b2",
                                "0x000000000000000000000000db5492265f6038831e89f495670ff909ade94bd9"
                            ],
                            "data": "0x000000000000000000000000000000000000000000000000000000000000018c00000000000000000000000000000000000000000000000000000000fc50010000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006402000000000000000000000000410a8b1502f01f2887f0b05b22cc934d45a628a72712124552435f53594d424f4c000000000000000000000000000000000000000000004d7945524332300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                            "logIndex": 20,
                            "blockHash": "0x974ba2485dbb37cf9253cec90bab96f96b806b1ee1db8f8d4833f69258f635d3"
                        }
                    ],
                    "blockNumber": 3957578,
                    "confirmations": 1,
                    "cumulativeGasUsed": {
                        "type": "BigNumber",
                        "hex": "0x2e6cd8"
                    },
                    "effectiveGasPrice": {
                        "type": "BigNumber",
                        "hex": "0x59689a64"
                    },
                    "status": 1,
                    "type": 2,
                    "byzantium": true,
                    "events": [
                        {
                            "transactionIndex": 20,
                            "blockNumber": 3957578,
                            "transactionHash": "0x62f2b7d16c483b3ec76962fb5337b6a442458af0027941e717770afbb3769b08",
                            "address": "0x4a8bc80Ed5a4067f1CCf107057b8270E0cC11A78",
                            "topics": [
                                "0x6eb224fb001ed210e379b335e35efe88672a8ce935d981a6896b27ffdf52a3b2",
                                "0x000000000000000000000000db5492265f6038831e89f495670ff909ade94bd9"
                            ],
                            "data": "0x000000000000000000000000000000000000000000000000000000000000018c00000000000000000000000000000000000000000000000000000000fc50010000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006402000000000000000000000000410a8b1502f01f2887f0b05b22cc934d45a628a72712124552435f53594d424f4c000000000000000000000000000000000000000000004d7945524332300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                            "logIndex": 20,
                            "blockHash": "0x974ba2485dbb37cf9253cec90bab96f96b806b1ee1db8f8d4833f69258f635d3"
                        }
                    ]
                },
                "emitterAddress": "000000000000000000000000410a8b1502f01f2887f0b05b22cc934d45a628a7",
                "sequence": "396"
            }
        }"#;

        async fn test(payload: Payload) -> Result<(String, String), reqwest::Error> {
            let client = reqwest::Client::new();
            let response = client
                .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/attest_from_eth")
                .json(&payload)
                .send()
                .await?
                .json::<ServerlessOutput>()
                .await?;

            let emitter = response.output.emitter_address;
            let sequence = response.output.sequence;

            Ok((emitter, sequence))
        }

        let payload = Payload {
            network_name: "devnet".into(),
            token: "0xDB5492265f6038831E89f495670FF909aDe94bd9".into(),
            keypair: "".into(),
        };

        let res = test(payload).await.unwrap();
        dbg!(res);
    }
}
