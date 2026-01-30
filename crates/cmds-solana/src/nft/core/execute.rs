use mpl_core::instructions::ExecuteV1Builder;
use tracing::info;

use crate::prelude::*;

// Command Name
const NAME: &str = "execute_instruction";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_execute_instruction.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub collection: Pubkey,
    pub instructions: Vec<Instruction>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let asset_signer = Pubkey::find_program_address(
        &["mpl-core-execute".as_bytes(), input.asset.as_ref()],
        &mpl_core::ID,
    )
    .0;

    info!("asset_signer: {:?}", asset_signer);

    let mut builder: ExecuteV1Builder = ExecuteV1Builder::new();

    let instructions: Vec<u8> = input
        .instructions
        .iter()
        .flat_map(|ix| ix.data.clone())
        .collect();

    let builder = builder
        .asset(input.asset)
        .asset_signer(asset_signer)
        .collection(Some(input.collection))
        .payer(input.fee_payer.pubkey())
        .system_program(solana_sdk_ids::system_program::id())
        .instruction_data(instructions);

    let ins = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into_iter().collect(),
        instructions: [ins].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    // account_info.resize(new_size)?;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mpl_core::{
        accounts::BaseAssetV1,
        fetch_external_plugin_adapter, fetch_external_plugin_adapter_data_info,
        types::{AppData, ExternalPluginAdapterKey, PluginAuthority},
    };
    use solana_program::account_info::IntoAccountInfo;
    use spl_record::state::RecordData;

    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[allow(unused)]
    #[tokio::test]
    #[ignore]
    async fn read_data() {
        tracing_subscriber::fmt::try_init().ok();

        let ctx = CommandContext::default();
        let rpc_client = ctx.solana_client();

        let asset = Pubkey::from_str("EWiBXUrzTomTWhsdzgthaW3s9UNsd9D7HhMxFFUvKeNZ").unwrap();

        let mut account = rpc_client.get_account(&asset).await.unwrap();

        // dbg!(&account);
        let account_data = bytemuck::try_from_bytes::<RecordData>(
            &account.data[..RecordData::WRITABLE_START_INDEX],
        )
        .unwrap();

        // dbg!(&account_data);
        let data = &account.data[RecordData::WRITABLE_START_INDEX..];
        // dbg!(&data);

        let data = String::from_utf8(data.to_vec())
            .unwrap()
            .trim_end_matches('\0')
            .to_string();
        // dbg!(&data);

        let asset_v1 = BaseAssetV1::from_bytes(&account.data).unwrap();

        let account_info = (&asset, &mut account).into_account_info();

        // Fetches the `AppData` plugin based on the Authority of the plugin.
        let plugin_key = ExternalPluginAdapterKey::AppData(PluginAuthority::Owner);

        let app_data_plugin = fetch_external_plugin_adapter::<BaseAssetV1, AppData>(
            &account_info,
            Some(&asset_v1),
            &plugin_key,
        )
        .unwrap();

        // dbg!(app_data_plugin);

        let (data_offset, data_length) =
            fetch_external_plugin_adapter_data_info(&account_info, Some(&asset_v1), &plugin_key)
                .unwrap();

        // grab app_data data from account_info
        let data = account_info.data.borrow()[data_offset..data_offset + data_length].to_vec();
        let account_data =
            bytemuck::try_from_bytes::<RecordData>(&data[..RecordData::WRITABLE_START_INDEX])
                .unwrap();
        // dbg!(&account_data);

        let data = &data[RecordData::WRITABLE_START_INDEX..];
        // dbg!(&data);

        let data = String::from_utf8(data.to_vec())
            .unwrap()
            .trim_end_matches('\0')
            .to_string();
        // dbg!(&data);

        // Deserialize the data
        // let data: String = bincode::deserialize::<String>(&data).unwrap();
        // dbg!(data);
    }
}
