use super::NftMetadata;
use crate::prelude::*;
use bundlr_sdk::{error::BundlrError, tags::Tag, Bundlr, Ed25519Signer};
use flow_lib::solana::SIGNATURE_TIMEOUT;
use std::collections::HashSet;

pub struct BundlrSigner {
    keypair: Keypair,
    ctx: Context,
}

impl BundlrSigner {
    pub fn new(keypair: Keypair, ctx: Context) -> Self {
        Self { keypair, ctx }
    }
}

impl bundlr_sdk::Signer for BundlrSigner {
    const SIG_TYPE: u16 = Ed25519Signer::SIG_TYPE;
    const SIG_LENGTH: u16 = Ed25519Signer::SIG_LENGTH;
    const PUB_LENGTH: u16 = Ed25519Signer::PUB_LENGTH;

    fn sign(&self, msg: bytes::Bytes) -> Result<bytes::Bytes, BundlrError> {
        let sig = if self.keypair.is_user_wallet() {
            let rt = self
                .ctx
                .get::<tokio::runtime::Handle>()
                .ok_or_else(|| BundlrError::SigningError("tokio runtime not found".to_owned()))?
                .clone();
            let ctx = self.ctx.clone();
            let pubkey = self.keypair.pubkey();
            rt.block_on(async move {
                tokio::time::timeout(
                    SIGNATURE_TIMEOUT,
                    ctx.request_signature(pubkey, msg, SIGNATURE_TIMEOUT),
                )
                .await
            })
            .map_err(|e| BundlrError::SigningError(e.to_string()))?
            .map_err(|e| BundlrError::SigningError(e.to_string()))?
        } else {
            self.keypair.sign_message(&msg)
        };
        Ok(<[u8; 64]>::from(sig).to_vec().into())
    }

    fn pub_key(&self) -> bytes::Bytes {
        self.keypair.pubkey().to_bytes().to_vec().into()
    }
}

#[derive(Debug, Clone)]
pub struct ArweaveNftUpload;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    pub metadata: NftMetadata,
    #[serde(default = "value::default::bool_true")]
    pub fund_bundlr: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub metadata_url: String,
    pub updated_metadata: NftMetadata,
}

const ARWEAVE_NFT_UPLOAD: &str = "arweave_nft_upload";

// Inputs
const FEE_PAYER: &str = "fee_payer";
const METADATA: &str = "metadata";
const FUND_BUNDLR: &str = "fund_bundlr";

// Outputs
const METADATA_URL: &str = "metadata_url";
const UPDATED_METADATA: &str = "updated_metadata";

#[async_trait]
impl CommandTrait for ArweaveNftUpload {
    fn name(&self) -> Name {
        ARWEAVE_NFT_UPLOAD.into()
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
                name: METADATA.into(),
                type_bounds: [ValueType::Free].to_vec(),
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
        [
            CmdOutput {
                name: METADATA_URL.into(),
                r#type: ValueType::String,
            },
            CmdOutput {
                name: UPDATED_METADATA.into(),
                r#type: ValueType::Free,
            },
        ]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input {
            fee_payer,
            mut metadata,
            fund_bundlr,
        } = value::from_map(inputs)?;

        let mut uploader = Uploader::new(
            ctx.solana_client.clone(),
            ctx.cfg.solana_client.cluster,
            fee_payer.clone_keypair(),
        )?;

        if fund_bundlr {
            uploader.lazy_fund_metadata(&metadata, &ctx).await?;
        }

        metadata.image = uploader.upload_file(ctx.clone(), &metadata.image).await?;

        if let Some(properties) = metadata.properties.as_mut() {
            if let Some(files) = properties.files.as_mut() {
                for file in files.iter_mut() {
                    file.uri = uploader.upload_file(ctx.clone(), &file.uri).await?;
                }
            }
        }

        let metadata_url = uploader
            .upload(
                ctx,
                serde_json::to_vec(&metadata).unwrap().into(),
                "application/json".to_owned(),
            )
            .await?;

        Ok(value::to_map(&Output {
            metadata_url,
            updated_metadata: metadata,
        })?)
    }
}

inventory::submit!(CommandDescription::new(ARWEAVE_NFT_UPLOAD, |_| Ok(
    Box::new(ArweaveNftUpload)
)));

pub(crate) struct Uploader {
    cache: HashMap<String, String>,
    content_cache: HashMap<String, bytes::Bytes>,
    fee_payer: Keypair,
    node_url: String,
    client: Arc<RpcClient>,
}

impl Uploader {
    pub fn new(
        client: Arc<RpcClient>,
        cluster: SolanaNet,
        fee_payer: Keypair,
    ) -> crate::Result<Uploader> {
        // Get Bundlr Network URL
        let node_url = match cluster {
            SolanaNet::Mainnet => "https://node1.bundlr.network".to_owned(),
            SolanaNet::Devnet => "https://devnet.bundlr.network".to_owned(),
            SolanaNet::Testnet => return Err(crate::Error::BundlrNotAvailableOnTestnet),
        };

        Ok(Uploader {
            cache: HashMap::new(),
            content_cache: HashMap::new(),
            fee_payer,
            node_url,
            client,
        })
    }

    pub async fn lazy_fund(&mut self, file_path: &str, signer: &Context) -> crate::Result<()> {
        let mut needed_size = self.get_file_size(file_path).await?;
        needed_size += 10_000;

        let needed_balance = self.get_price(needed_size).await?;
        let needed_balance = needed_balance + needed_balance / 10;

        let current_balance = self.get_current_balance().await?;

        if current_balance < needed_balance {
            self.fund(needed_balance - current_balance, signer).await?;
        }

        Ok(())
    }

    pub async fn lazy_fund_metadata(
        &mut self,
        metadata: &NftMetadata,
        signer: &Context,
    ) -> crate::Result<()> {
        let mut processed = HashSet::new();
        let mut needed_size = 0;

        let metadata_size = serde_json::to_vec(metadata).unwrap().len() as u64;

        needed_size += metadata_size;
        needed_size += self.get_file_size(&metadata.image).await?;
        processed.insert(metadata.image.clone());

        if let Some(properties) = metadata.properties.as_ref() {
            if let Some(files) = properties.files.as_ref() {
                for file in files.iter() {
                    if processed.contains(&file.uri) {
                        continue;
                    }

                    needed_size += self.get_file_size(&file.uri).await?;
                    processed.insert(file.uri.clone());
                }
            }
        }

        needed_size += 100_000; // tx_fee + some offset
        needed_size += metadata_size * 4 / 10; // metadata change offset

        let needed_balance = self.get_price(needed_size).await?;
        let needed_balance = needed_balance + needed_balance / 10;

        let current_balance = self.get_current_balance().await?;

        if current_balance < needed_balance {
            self.fund(needed_balance - current_balance, signer).await?;
        }

        Ok(())
    }

    async fn get_file(&mut self, path: &str) -> crate::Result<bytes::Bytes> {
        if let Some(content) = self.content_cache.get(path) {
            Ok(content.clone())
        } else {
            let resp = reqwest::get(path).await?;
            let data = resp.bytes().await?;
            self.content_cache.insert(path.to_owned(), data.clone());
            Ok(data)
        }
    }

    async fn get_file_size(&mut self, path: &str) -> crate::Result<u64> {
        Ok(self.get_file(path).await?.len() as u64)
    }

    async fn get_price(&self, size: u64) -> crate::Result<u64> {
        let resp = reqwest::get(format!("{}/price/solana/{}", &self.node_url, size)).await?;
        let text = resp.text().await?;
        text.parse::<u64>()
            .map_err(|_| crate::Error::BundlrApiInvalidResponse(text.clone()))
    }

    async fn get_current_balance(&self) -> crate::Result<u64> {
        #[serde_with::serde_as]
        #[derive(Deserialize)]
        struct Resp {
            #[serde_as(as = "serde_with::DisplayFromStr")]
            balance: u64,
        }

        let resp = reqwest::get(format!(
            "{}/account/balance/solana/?address={}",
            &self.node_url,
            self.fee_payer.pubkey()
        ))
        .await?;

        if resp.status().is_success() {
            let resp = resp.json::<Resp>().await?;
            Ok(resp.balance)
        } else {
            let text = resp.text().await?;
            Err(crate::Error::BundlrApiInvalidResponse(text))
        }
    }

    async fn fund(&self, amount: u64, signer: &Context) -> crate::Result<()> {
        #[derive(Deserialize, Serialize)]
        struct Addresses {
            solana: String,
        }

        #[derive(Deserialize, Serialize)]
        struct Info {
            addresses: Addresses,
        }

        let resp = reqwest::get(format!("{}/info", &self.node_url)).await?;

        let info: Info = serde_json::from_str(&resp.text().await?)?;

        let recipient = info
            .addresses
            .solana
            .parse::<Pubkey>()
            .map_err(crate::Error::custom)?;

        let instruction =
            solana_sdk::system_instruction::transfer(&self.fee_payer.pubkey(), &recipient, amount);
        let (mut tx, recent_blockhash) =
            execute(&self.client, &self.fee_payer.pubkey(), &[instruction], 0).await?;

        try_sign_wallet(signer, &mut tx, &[&self.fee_payer], recent_blockhash).await?;

        let signature = submit_transaction(&self.client, tx).await?;

        let resp = reqwest::Client::new()
            .post(format!("{}/account/balance/solana", &self.node_url))
            .json(&serde_json::json!({
                "tx_id": signature.to_string(),
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(crate::Error::BundlrTxRegisterFailed(signature.to_string()));
        }

        Ok(())
    }

    pub async fn upload_file(&mut self, ctx: Context, file_path: &str) -> crate::Result<String> {
        if let Some(url) = self.cache.get(file_path) {
            return Ok(url.clone());
        }

        let content_type = mime_guess::from_path(file_path)
            .first()
            .ok_or(crate::Error::MimeTypeNotFound)?
            .to_string();
        let data = self.get_file(file_path).await?;

        let url = self.upload(ctx, data, content_type).await?;

        self.cache.insert(file_path.to_owned(), url.clone());

        Ok(url)
    }

    pub async fn upload(
        &self,
        ctx: Context,
        data: bytes::Bytes,
        content_type: String,
    ) -> crate::Result<String> {
        let bundlr = Bundlr::new(
            self.node_url.clone(),
            "solana".to_string(),
            "sol".to_string(),
            BundlrSigner::new(self.fee_payer.clone_keypair(), ctx),
        );

        let (bundlr, tx) = tokio::task::spawn_blocking(move || {
            let tx = bundlr.create_transaction_with_tags(
                data.to_vec(),
                vec![Tag::new("Content-Type".into(), content_type)],
            );
            (bundlr, tx)
        })
        .await
        .map_err(|_| {
            crate::Error::custom(anyhow::anyhow!(
                "failed to create and sign bundlr transaction"
            ))
        })?;

        let resp: BundlrResponse = serde_json::from_value(bundlr.send_transaction(tx).await?)?;

        Ok(format!("https://arweave.net/{}", resp.id))
    }
}

#[derive(Deserialize)]
struct BundlrResponse {
    id: String,
}
