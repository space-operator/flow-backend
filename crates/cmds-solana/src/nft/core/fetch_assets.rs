use mpl_core::{accounts::BaseAssetV1, types::Key};
use solana_account_decoder::UiAccount;
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};

use crate::prelude::*;

// Command Name
const NAME: &str = "fetch_assets";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/fetch_assets.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(default, with = "value::pubkey")]
    pub collection: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub assets: Vec<BaseAssetV1>,
}

fn parse_account(account: UiAccount) -> Result<BaseAssetV1, CommandError> {
    let data = account
        .data
        .decode()
        .ok_or_else(|| CommandError::msg("could not decode account data"))?;
    Ok(BaseAssetV1::from_bytes(&data)?)
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let collection = input.collection;
    tracing::debug!("Collection {:?}", collection);

    let rpc_data = ctx
        .solana_client()
        .get_program_ui_accounts_with_config(
            &mpl_core::ID,
            RpcProgramAccountsConfig {
                account_config: RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    ..Default::default()
                },
                filters: Some(vec![
                    RpcFilterType::Memcmp(Memcmp::new(
                        0,
                        MemcmpEncodedBytes::Bytes([Key::AssetV1 as u8].into()),
                    )),
                    RpcFilterType::Memcmp(Memcmp::new(
                        34,
                        MemcmpEncodedBytes::Bytes([2_u8].into()),
                    )),
                    RpcFilterType::Memcmp(Memcmp::new(
                        35,
                        MemcmpEncodedBytes::Base58(collection.to_string()),
                    )),
                ]),
                ..Default::default()
            },
        )
        .await?;

    let assets = rpc_data
        .into_iter()
        .map(|(_, account)| parse_account(account))
        .collect::<Result<Vec<_>, _>>()?;

    tracing::debug!("Assets {:?}", assets);

    Ok(Output { assets })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
}
