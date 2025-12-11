use mpl_core::{
    instructions::WriteExternalPluginAdapterDataV1Builder,
    types::{ExternalPluginAdapterKey, PluginAuthority},
};

use crate::prelude::*;

// Command Name
const NAME: &str = "write_app_data";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_write_app_data.json");

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
    #[serde_as(as = "Option<AsPubkey>")]
    pub buffer: Option<Pubkey>,
    pub authority: Wallet,
    pub data: Option<String>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // let data_len = data.len();

    // // 0. get the asset account
    // let mut account = ctx.solana_client().get_account(&input.asset).await.unwrap();

    // let asset_v1 = BaseAssetV1::from_bytes(&account.data).unwrap();

    // let old_size = account.data.len();
    // let account_info = (&input.asset, &mut account).into_account_info();

    // let mut fee_payer = ctx
    //     .solana_client()
    //     .get_account(&input.fee_payer.pubkey())
    //     .await
    //     .unwrap();
    // let fee_payer_info = (&input.fee_payer.pubkey(), &mut fee_payer).into_account_info();

    // let mut system_program = ctx
    //     .solana_client()
    //     .get_account(&solana_sdk_ids::system_program::id())
    //     .await
    //     .unwrap();
    // let system_program_info =
    //     (&solana_sdk_ids::system_program::id(), &mut system_program).into_account_info();

    // let new_size = old_size
    //     .checked_add(data_len)
    //     .ok_or(anyhow::anyhow!("NumericalOverflow"))?;

    // let resize_instruction = if new_size > old_size {
    //     let rent = Rent::get()?;
    //     let new_minimum_balance = rent.minimum_balance(new_size);
    //     let lamports_diff = new_minimum_balance.abs_diff(account_info.lamports());

    //     let transfer_sol = system_instruction::transfer(
    //         &input.fee_payer.pubkey(),
    //         &account_info.key,
    //         lamports_diff,
    //     );
    //     Some(instruction)

    //     // resize_or_reallocate_account_raw(
    //     //     &account_info,
    //     //     &fee_payer_info,
    //     //     &system_program_info,
    //     //     new_size,
    //     // )?;
    // } else {
    //     None
    // };

    // 1. create a buffer account and chunk the data and write to the buffer account
    // https://github.com/metaplex-foundation/mpl-inscription/blob/main/programs/inscription/src/processor/write_data.rs
    // const CHUNK_SIZE: usize = 800;

    // chunk the data
    // let chunks = data.chunks(CHUNK_SIZE);
    // for chunk in chunks {
    // write

    // from inscription
    // let old_size = ctx.accounts.inscription_account.data_len();
    // let write_end = args
    //     .offset
    //     .checked_add(args.value.len())
    //     .ok_or(MplInscriptionError::NumericalOverflow)?;

    // // Resize the account to fit the new data if necessary.
    // if write_end > old_size {
    //     resize_or_reallocate_account_raw(
    //         ctx.accounts.inscription_account,
    //         ctx.accounts.payer,
    //         ctx.accounts.system_program,
    //         write_end,
    //     )?;
    // }

    // // Write the inscription metadata to the metadata account.
    // sol_memcpy(
    //     &mut ctx.accounts.inscription_account.try_borrow_mut_data()?[args.offset..],
    //     &args.value,
    //     args.value.len(),
    // );

    // from appdata
    //     let new_size = account
    //     .data_len()
    //     .checked_add(size_increase)
    //     .ok_or(MplCoreError::NumericalOverflow)?;

    // resize_or_reallocate_account(account, payer, system_program, new_size)?;
    // plugin_header.save(account, header_offset)?;
    // plugin.save(account, old_registry_offset)?;

    // if let Some(data) = appended_data {
    //     sol_memcpy(
    //         &mut account.data.borrow_mut()[data_offset..],
    //         data,
    //         data.len(),
    //     );
    // };
    // }
    // 2. create an updated instruction to replace WriteExternalPluginAdapterDataV1Builder
    // https://github.com/metaplex-foundation/mpl-core/blob/main/programs/mpl-core/src/processor/write_external_plugin_adapter_data.rs

    // calc new size
    // let mut account = ctx.solana_client().get_account(&input.asset).await.unwrap();
    // let account_info = (&input.asset, &mut account).into_account_info();

    // let (asset, mut header, mut registry) = fetch_core_data::<AssetV1>(&account_info)?;

    // let (record, plugin) = fetch_wrapped_external_plugin_adapter::<AssetV1>(
    //     &account_info,
    //     None,
    //     &ExternalPluginAdapterKeyAlias::AppData(Authority::Owner),
    // )?;

    // // Extract the data offset and data length as they should always be set.
    // let data_len = record.data_len.ok_or(anyhow::anyhow!("InvalidPlugin"))?;
    // let new_data_len = input.data.as_ref().unwrap().len();
    // let size_diff = (new_data_len as isize)
    //     .checked_sub(data_len as isize)
    //     .ok_or(anyhow::anyhow!("NumericalOverflow"))?;

    // let new_size = (account_info.data_len() as isize)
    //     .checked_add(size_diff)
    //     .ok_or(anyhow::anyhow!("NumericalOverflow"))?;

    let mut builder: WriteExternalPluginAdapterDataV1Builder =
        WriteExternalPluginAdapterDataV1Builder::new();

    let builder = builder
        .asset(input.asset)
        .collection(Some(input.collection))
        .payer(input.fee_payer.pubkey())
        .authority(Some(input.authority.pubkey()))
        .system_program(solana_sdk_ids::system_program::id())
        .log_wrapper(None)
        .key(ExternalPluginAdapterKey::AppData(PluginAuthority::Owner));

    // check data
    let builder = if let Some(data) = input.data {
        let data = bincode::serialize(&data).unwrap();
        builder.data(data)
    } else {
        builder
    };

    // check buffer
    let builder = if let Some(buffer) = input.buffer {
        builder.buffer(Some(buffer))
    } else {
        builder
    };

    let ins = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into_iter().collect(),
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
        accounts::BaseAssetV1, fetch_external_plugin_adapter,
        fetch_external_plugin_adapter_data_info, types::AppData,
    };
    use solana_program::account_info::IntoAccountInfo;
    use spl_record::state::RecordData;

    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
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
