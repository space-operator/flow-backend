use crate::{prelude::*, wormhole::WormholeResponse};

use std::time::Duration;
use tokio::time::sleep;

// Command Name
const NAME: &str = "get_vaa";

const DEFINITION: &str = include_str!("../../../../node-definitions/solana/wormhole/get_vaa.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: Lazy<Result<CmdBuilder, BuilderError>> =
        Lazy::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub emitter: String,
    pub chain_id: String,
    pub sequence: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    response: Option<WormholeResponse>,
    vaa: Option<String>,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    let wormhole_endpoint = match ctx.cfg.solana_client.cluster {
        SolanaNet::Mainnet => "",
        SolanaNet::Testnet => "",
        SolanaNet::Devnet => "https://api.testnet.wormscan.io",
    }
    .to_owned();

    let wormhole_path: &str = "api/v1/vaas";

    let wormhole_url = format!(
        "{}/{}/{}/{}/{}",
        wormhole_endpoint, wormhole_path, input.chain_id, input.emitter, input.sequence
    );

    async fn send_wormhole_request(
        client: &reqwest::Client,
        wormhole_url: &str,
        timeout: Duration,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let response = client.get(wormhole_url).timeout(timeout).send().await?;
        Ok(response)
    }

    let timeout = Duration::from_secs(60);

    let mut response = send_wormhole_request(&ctx.http, &wormhole_url, timeout).await?;

    while response.status() != 200 {
        // Solana
        if input.chain_id == "1" {
            sleep(Duration::from_secs(5)).await;
        }
        // Eth Sepolia about 20m
        if input.chain_id == "10002" {
            sleep(Duration::from_secs(45)).await;
        }
        response = send_wormhole_request(&ctx.http, &wormhole_url, timeout).await?;
    }

    let response_text = response.text().await?;
    let response: WormholeResponse = serde_json::from_str(&response_text)?;

    let vaa = &response.data.vaa;

    Ok(Output {
        response: Some(response.clone()),
        vaa: Some(vaa.to_owned()),
    })
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        // const res:&str = "{\"data\":{\"sequence\":420,\"id\":\"10002/000000000000000000000000db5492265f6038831e89f495670ff909ade94bd9/420\",\"version\":1,\"emitterChain\":10002,\"emitterAddr\":\"000000000000000000000000db5492265f6038831e89f495670ff909ade94bd9\",\"guardianSetIndex\":0,\"vaa\":\"AQAAAAABAIGVMaxqz2cou11lb1AVxzNNzPAV9ooflmTPSmcQmChxEfwlzHd+osaDIilfFlxNW7g5IMQPqQDhkgTyU/46qDwAZMBlwLQtAQAnEgAAAAAAAAAAAAAAANtUkiZfYDiDHon0lWcP+Qmt6UvZAAAAAAAAAaQBAgAAAAAAAAAAAAAAAEEKixUC8B8oh/CwWyLMk01FpiinJxISRVJDX1NZTUJPTAAAAAAAAAAAAAAAAAAAAAAAAAAAAABNeUVSQzIwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==\",\"timestamp\":\"2023-07-26T00:16:00Z\",\"updatedAt\":\"2023-07-26T00:33:35.942Z\",\"indexedAt\":\"2023-07-26T00:33:35.942Z\",\"txHash\":\"0eacb8738102df585cb5dbbd7664f8e2fd9e04c02bcb7080cdc62b9bfcf09d9d\"},\"pagination\":{\"next\":\"\"}}";
        // let response: WormholeResponse = ron::de::from_str(&res).unwrap();

        // dbg!(response.clone());
    }
}
*/
