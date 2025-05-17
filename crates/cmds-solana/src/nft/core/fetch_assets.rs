use mpl_core::{accounts::BaseAssetV1, types::Key};
use solana_rpc_client_api::{
    config::RpcProgramAccountsConfig,
    filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use tracing::info;

use crate::prelude::*;

// Command Name
const NAME: &str = "fetch_assets";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/fetch_assets.json");

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

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // let rpc_data = ctx
    //     .solana_client()
    //     .get_account_data(&input.collection)
    //     .await
    //     .unwrap();

    let collection = input.collection;
    info!("Collection {:?}", collection);

    let rpc_data = ctx
        .solana_client()
        .get_program_accounts_with_config(
            &mpl_core::ID,
            RpcProgramAccountsConfig {
                filters: Some(vec![
                    RpcFilterType::Memcmp(Memcmp::new(
                        0,
                        MemcmpEncodedBytes::Bytes(vec![Key::AssetV1 as u8]),
                    )),
                    RpcFilterType::Memcmp(Memcmp::new(34, MemcmpEncodedBytes::Bytes(vec![2_u8]))),
                    RpcFilterType::Memcmp(Memcmp::new(
                        35,
                        MemcmpEncodedBytes::Base58(collection.to_string()),
                    )),
                ]),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let accounts_iter = rpc_data.into_iter().map(|(_, account)| account);

    let mut assets: Vec<BaseAssetV1> = vec![];

    for account in accounts_iter {
        info!("Account {:?}", account);
        let asset: BaseAssetV1 = BaseAssetV1::from_bytes(&account.data).unwrap();
        assets.push(asset);
    }
    info!("Assets {:?}", assets);

    Ok(Output { assets })
}
