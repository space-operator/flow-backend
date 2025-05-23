use crate::prelude::*;

use super::{RecordData, pod_from_bytes};

pub const NAME: &str = "read_record";

const DEFINITION: &str = flow_lib::node_definition!("/record/read_record.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize)]
struct Input {
    #[serde(with = "value::pubkey")]
    account: Pubkey,
}

#[derive(Serialize)]
struct Output {
    #[serde(with = "value::pubkey")]
    authority: Pubkey,
    version: u8,
    data: String,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let resp = ctx.solana_client().get_account(&input.account).await?;

    let account_data = pod_from_bytes::<RecordData>(&resp.data[..RecordData::WRITABLE_START_INDEX])
        .map_err(|_| {
            crate::error::Error::Any(anyhow::anyhow!(
                "Error: Invalid account data: {}",
                input.account
            ))
        })?;

    Ok(Output {
        authority: account_data.authority,
        version: account_data.version,
        data: String::from_utf8(resp.data[RecordData::WRITABLE_START_INDEX..].to_vec())?,
    })
}
