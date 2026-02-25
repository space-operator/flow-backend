use super::arweave_nft_upload::Uploader;
use crate::prelude::*;

const NAME: &str = "arweave_file_upload";

const DEFINITION: &str = flow_lib::node_definition!("arweave/arweave_file_upload.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub file_path: String,
    #[serde(default = "value::default::bool_true")]
    pub fund_bundlr: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub file_url: String,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut uploader = Uploader::new(
        ctx.solana_client().clone(),
        ctx.solana_config().cluster,
        input.fee_payer,
    )?;

    if input.fund_bundlr {
        uploader.lazy_fund(&input.file_path, &mut ctx).await?;
    }

    let file_url = uploader.upload_file(ctx, &input.file_path).await?;

    Ok(Output { file_url })
}
