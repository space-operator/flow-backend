use crate::{prelude::*, wormhole::token_bridge::eth::GetForeignAddress};

// Command Name
const NAME: &str = "get_foreign_asset_eth";

const DEFINITION: &str = include_str!(
    "../../../../../node-definitions/solana/wormhole/utils/get_foreign_asset_eth.json"
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
    pub network: String,
    pub token: String,
    pub is_nft: bool,
    pub chain_id: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    address: String,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    #[derive(Serialize, Deserialize, Debug)]
    struct Payload {
        #[serde(rename = "networkName")]
        network: String,
        token: String,
        #[serde(rename = "isNFT")]
        is_nft: bool,
        #[serde(rename = "chainId")]
        chain_id: u16,
    }

    let payload = Payload {
        network: input.network,
        token: input.token,
        is_nft: input.is_nft,
        chain_id: input.chain_id,
    };

    let response: GetForeignAddress = ctx
        .http
        .post("https://gygvoikm3c.execute-api.us-east-1.amazonaws.com/get_foreign_asset_eth")
        .json(&payload)
        .send()
        .await?
        .json::<GetForeignAddress>()
        .await?;

    Ok(Output {
        address: response.output.address,
    })
}
