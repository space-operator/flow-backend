use crate::mpl_token_metadata::NftMetadata;
use crate::prelude::*;
use bundlr_sdk::{Bundlr, Ed25519Signer, error::BundlrError, tags::Tag};
use flow_lib::solana::{Keypair, SIGNATURE_TIMEOUT};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_message::{VersionedMessage, v0};
use solana_presigner::Presigner;
use solana_transaction::versioned::VersionedTransaction;
use std::collections::HashSet;

pub struct BundlrSigner {
    keypair: Wallet,
    ctx: CommandContext,
}

impl BundlrSigner {
    pub fn new(keypair: Wallet, ctx: CommandContext) -> Self {
        Self { keypair, ctx }
    }
}

impl bundlr_sdk::Signer for BundlrSigner {
    const SIG_TYPE: u16 = Ed25519Signer::SIG_TYPE;
    const SIG_LENGTH: u16 = Ed25519Signer::SIG_LENGTH;
    const PUB_LENGTH: u16 = Ed25519Signer::PUB_LENGTH;

    fn sign(&self, msg: bytes::Bytes) -> Result<bytes::Bytes, BundlrError> {
        let sig = match self.keypair.keypair() {
            Some(keypair) => keypair.sign_message(&msg),
            _ => {
                let rt = self
                    .ctx
                    // TODO: store this officially instead of extension
                    .get::<tokio::runtime::Handle>()
                    .ok_or_else(|| BundlrError::SigningError("tokio runtime not found".to_owned()))?
                    .clone();
                let mut ctx = self.ctx.clone();
                let pubkey = self.keypair.pubkey();
                let token = self.keypair.token();
                rt.block_on(async move {
                    tokio::time::timeout(
                        SIGNATURE_TIMEOUT,
                        ctx.request_signature(pubkey, token, msg, SIGNATURE_TIMEOUT),
                    )
                    .await
                    .map(|res| res.map(|res| res.signature))
                })
                .map_err(|e| BundlrError::SigningError(e.to_string()))?
                .map_err(|e| BundlrError::SigningError(e.to_string()))?
            }
        };
        Ok(<[u8; 64]>::from(sig).to_vec().into())
    }

    fn pub_key(&self) -> bytes::Bytes {
        self.keypair.pubkey().to_bytes().to_vec().into()
    }
}

const NAME: &str = "arweave_nft_upload";

const DEFINITION: &str = flow_lib::node_definition!("arweave/arweave_nft_upload.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub metadata: NftMetadata,
    #[serde(default = "value::default::bool_true")]
    pub fund_bundlr: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub metadata_url: String,
    pub updated_metadata: NftMetadata,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Generate ephemeral keypair for Bundlr when fee_payer is an adapter wallet.
    // This signs Arweave data items server-side instead of forwarding raw deep
    // hashes to the frontend (which wallets like Phantom reject via signMessage).
    let ephemeral_keypair = if input.fee_payer.is_adapter_wallet() {
        Some(Keypair::new())
    } else {
        None
    };

    let mut uploader = Uploader::new(
        ctx.solana_client().clone(),
        ctx.solana_config().cluster,
        input.fee_payer,
        ephemeral_keypair,
    )?;

    if input.fund_bundlr {
        uploader.lazy_fund_metadata(&input.metadata, &mut ctx).await?;
    }

    let mut metadata = input.metadata;
    metadata.image = uploader.upload_file(ctx.clone(), &metadata.image).await?;

    if let Some(properties) = metadata.properties.as_mut()
        && let Some(files) = properties.files.as_mut()
    {
        for file in files.iter_mut() {
            file.uri = uploader.upload_file(ctx.clone(), &file.uri).await?;
        }
    }

    let metadata_url = uploader
        .upload(
            ctx,
            serde_json::to_vec(&metadata).unwrap().into(),
            "application/json".to_owned(),
        )
        .await?;

    Ok(Output {
        metadata_url,
        updated_metadata: metadata,
    })
}

pub(crate) struct Uploader {
    cache: HashMap<String, String>,
    content_cache: HashMap<String, bytes::Bytes>,
    fee_payer: Wallet,
    /// Ephemeral keypair used for Bundlr signing when fee_payer is an adapter wallet.
    /// Signs Arweave data items locally, avoiding forwarding raw deep hashes to the frontend.
    ephemeral_keypair: Option<Keypair>,
    node_url: String,
    gateway_url: String,
    client: Arc<RpcClient>,
}

impl Uploader {
    pub fn new(
        client: Arc<RpcClient>,
        cluster: SolanaNet,
        fee_payer: Wallet,
        ephemeral_keypair: Option<Keypair>,
    ) -> crate::Result<Uploader> {
        // Get Bundlr Network URL
        let (node_url, gateway_url) = match cluster {
            SolanaNet::Mainnet => (
                "https://node1.bundlr.network".to_owned(),
                "https://arweave.net".to_owned(),
            ),
            SolanaNet::Devnet => (
                "https://devnet.bundlr.network".to_owned(),
                "https://devnet.irys.xyz".to_owned(),
            ),
            SolanaNet::Testnet => return Err(crate::Error::BundlrNotAvailableOnTestnet),
        };

        Ok(Uploader {
            cache: HashMap::new(),
            content_cache: HashMap::new(),
            fee_payer,
            ephemeral_keypair,
            node_url,
            gateway_url,
            client,
        })
    }

    /// Pubkey used for Bundlr account operations.
    /// Returns the ephemeral keypair's pubkey when fee_payer is an adapter wallet.
    fn bundlr_pubkey(&self) -> Pubkey {
        match &self.ephemeral_keypair {
            Some(kp) => kp.pubkey(),
            None => self.fee_payer.pubkey(),
        }
    }

    /// Wallet used for Bundlr signing.
    /// Returns a Wallet::Keypair wrapping the ephemeral key so BundlrSigner signs locally.
    fn bundlr_wallet(&self) -> Wallet {
        match &self.ephemeral_keypair {
            Some(kp) => Wallet::Keypair(kp.insecure_clone()),
            None => self.fee_payer.clone(),
        }
    }

    pub async fn lazy_fund(
        &mut self,
        file_path: &str,
        signer: &mut CommandContext,
    ) -> crate::Result<()> {
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
        signer: &mut CommandContext,
    ) -> crate::Result<()> {
        let mut processed = HashSet::new();
        let mut needed_size = 0;

        let metadata_size = serde_json::to_vec(metadata).unwrap().len() as u64;

        needed_size += metadata_size;
        needed_size += self.get_file_size(&metadata.image).await?;
        processed.insert(metadata.image.clone());

        if let Some(properties) = metadata.properties.as_ref()
            && let Some(files) = properties.files.as_ref()
        {
            for file in files.iter() {
                if processed.contains(&file.uri) {
                    continue;
                }

                needed_size += self.get_file_size(&file.uri).await?;
                processed.insert(file.uri.clone());
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
        match self.content_cache.get(path) {
            Some(content) => Ok(content.clone()),
            _ => {
                let resp = reqwest::get(path).await?;
                let data = resp.bytes().await?;
                self.content_cache.insert(path.to_owned(), data.clone());
                Ok(data)
            }
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
            self.bundlr_pubkey()
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

    async fn fund(&self, amount: u64, signer: &mut CommandContext) -> crate::Result<()> {
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

        let bundlr_address = info
            .addresses
            .solana
            .parse::<Pubkey>()
            .map_err(crate::Error::custom)?;

        // When using an ephemeral keypair, we need two-step funding:
        //   1. fee_payer → ephemeral (adapter-signed V0 Solana tx)
        //   2. ephemeral → Bundlr (locally-signed Solana tx)
        // Bundlr credits the SENDER of the SOL transfer, so the ephemeral key
        // must be the one that sends SOL to Bundlr.
        let funding_signature = if let Some(ephemeral) = &self.ephemeral_keypair {
            // Ephemeral accounts must hold >= rent-exempt minimum to exist on-chain.
            // After step 2, we drain the leftover back to fee_payer so the account
            // ends at exactly 0 lamports (garbage collected).
            let rent_exempt_min = self
                .client
                .get_minimum_balance_for_rent_exemption(0)
                .await?;
            let step2_tx_fee = 5000u64; // base fee for 1-signature legacy tx

            // Step 1: fee_payer → ephemeral (signed by adapter wallet via frontend)
            // Uses V0 message with pre-added compute budget instructions so the
            // wallet updates values in-place rather than adding new instructions
            // (which would change the message structure and fail is_same_message_logic).
            let step1_ix = solana_system_interface::instruction::transfer(
                &self.fee_payer.pubkey(),
                &ephemeral.pubkey(),
                amount + rent_exempt_min,
            );
            let blockhash1 = self.client.get_latest_blockhash().await?;
            let v0_msg = v0::Message::try_compile(
                &self.fee_payer.pubkey(),
                &[
                    ComputeBudgetInstruction::set_compute_unit_limit(200_000),
                    ComputeBudgetInstruction::set_compute_unit_price(1000),
                    step1_ix,
                ],
                &[], // no address lookup tables
                blockhash1,
            )
            .map_err(crate::Error::custom)?;
            let versioned_msg = VersionedMessage::V0(v0_msg);
            let msg_bytes: bytes::Bytes =
                bincode::serialize(&versioned_msg)
                    .map_err(crate::Error::custom)?
                    .into();

            let sig_resp = tokio::time::timeout(
                crate::utils::SIGNATURE_TIMEOUT,
                signer.request_signature(
                    self.fee_payer.pubkey(),
                    self.fee_payer.token(),
                    msg_bytes,
                    crate::utils::SIGNATURE_TIMEOUT,
                ),
            )
            .await
            .map_err(|_| crate::Error::SignatureTimeout)??;

            // Use the wallet's modified message if it changed the transaction
            let final_msg = match sig_resp.new_message {
                Some(ref new_msg) => bincode::deserialize::<VersionedMessage>(new_msg)
                    .map_err(crate::Error::custom)?,
                None => versioned_msg,
            };
            let presigner =
                Presigner::new(&self.fee_payer.pubkey(), &sig_resp.signature);
            let vtx = VersionedTransaction::try_new(final_msg, &[&presigner])
                .map_err(crate::Error::custom)?;

            self.client
                .send_and_confirm_transaction(&vtx)
                .await?;

            // Step 2: ephemeral → Bundlr + drain remainder back to fee_payer.
            // After paying step2_tx_fee, ephemeral has: amount + rent_exempt_min - step2_tx_fee
            // ix1 sends `amount` to Bundlr, ix2 drains the rest → ephemeral ends at 0.
            let drain_amount = rent_exempt_min.saturating_sub(step2_tx_fee);
            let step2_transfer_ix = solana_system_interface::instruction::transfer(
                &ephemeral.pubkey(),
                &bundlr_address,
                amount,
            );
            let step2_drain_ix = solana_system_interface::instruction::transfer(
                &ephemeral.pubkey(),
                &self.fee_payer.pubkey(),
                drain_amount,
            );
            let (mut tx2, blockhash2) = execute(
                &self.client,
                &ephemeral.pubkey(),
                &[step2_transfer_ix, step2_drain_ix],
            )
            .await?;
            tx2.try_sign(&[ephemeral], blockhash2)?;
            submit_transaction(&self.client, tx2).await?
        } else {
            // Direct funding: fee_payer → Bundlr
            let instruction = solana_system_interface::instruction::transfer(
                &self.fee_payer.pubkey(),
                &bundlr_address,
                amount,
            );
            let (mut tx, recent_blockhash) =
                execute(&self.client, &self.fee_payer.pubkey(), &[instruction]).await?;
            try_sign_wallet(signer, &mut tx, &self.fee_payer, recent_blockhash).await?;
            submit_transaction(&self.client, tx).await?
        };

        // Register the funding transaction with Bundlr
        let resp = reqwest::Client::new()
            .post(format!("{}/account/balance/solana", &self.node_url))
            .json(&serde_json::json!({
                "tx_id": funding_signature.to_string(),
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(crate::Error::BundlrTxRegisterFailed(
                funding_signature.to_string(),
            ));
        }

        Ok(())
    }

    pub async fn upload_file(
        &mut self,
        ctx: CommandContext,
        file_path: &str,
    ) -> crate::Result<String> {
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
        ctx: CommandContext,
        data: bytes::Bytes,
        content_type: String,
    ) -> crate::Result<String> {
        let bundlr = Bundlr::new(
            self.node_url.clone(),
            "solana".to_string(),
            "sol".to_string(),
            BundlrSigner::new(self.bundlr_wallet(), ctx),
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

        Ok(format!("{}/{}", self.gateway_url, resp.id))
    }
}

#[derive(Deserialize)]
struct BundlrResponse {
    id: String,
}
