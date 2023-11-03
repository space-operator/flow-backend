use super::arweave_nft_upload::Uploader;
use crate::prelude::*;

#[derive(Debug)]
pub struct ArweaveFileUpload;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    pub file_path: String,
    #[serde(default = "value::default::bool_true")]
    pub fund_bundlr: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub file_url: String,
}

const ARWEAVE_FILE_UPLOAD: &str = "arweave_file_upload";

// Inputs
const FEE_PAYER: &str = "fee_payer";
const FILE_PATH: &str = "file_path";
const FUND_BUNDLR: &str = "fund_bundlr";

// Outputs
const FILE_URL: &str = "file_url";

#[async_trait]
impl CommandTrait for ArweaveFileUpload {
    fn name(&self) -> Name {
        ARWEAVE_FILE_UPLOAD.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: FEE_PAYER.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: FILE_PATH.into(),
                type_bounds: [ValueType::String].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: FUND_BUNDLR.into(),
                type_bounds: [ValueType::Bool].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: FILE_URL.into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input {
            fee_payer,
            file_path,
            fund_bundlr,
        } = value::from_map(inputs)?;

        let mut uploader = Uploader::new(
            ctx.solana_client.clone(),
            ctx.cfg.solana_client.cluster,
            fee_payer.clone_keypair(),
        )?;

        if fund_bundlr {
            uploader.lazy_fund(&file_path, &ctx).await?;
        }

        let file_url = uploader.upload_file(ctx, &file_path).await?;

        Ok(value::to_map(&Output { file_url })?)
    }
}

inventory::submit!(CommandDescription::new(ARWEAVE_FILE_UPLOAD, |_| Ok(
    Box::new(ArweaveFileUpload)
)));
