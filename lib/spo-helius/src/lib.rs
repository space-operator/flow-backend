use anyhow::{bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_with::skip_serializing_none;
use std::sync::atomic::AtomicU64;

#[derive(Debug)]
pub struct Helius {
    client: reqwest::Client,
    mainnet_url: String,
    devnet_url: String,
    id: AtomicU64,
}

pub fn is_pubkey(s: &str) -> Result<&str, anyhow::Error> {
    let mut buf = [0u8; 32];
    let written = bs58::decode(s).into(&mut buf)?;
    ensure!(written == buf.len(), "invalid pubkey");
    Ok(s)
}

#[skip_serializing_none]
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetPriorityFeeEstimateRequest {
    pub transaction: Option<String>,
    pub account_keys: Option<Vec<String>>,
    pub options: Option<GetPriorityFeeEstimateOptions>,
}

#[skip_serializing_none]
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetPriorityFeeEstimateOptions {
    pub priority_level: Option<PriorityLevel>,
    pub include_all_priority_fee_levels: Option<bool>,
    pub transaction_encoding: Option<String>,
    pub lookback_slots: Option<u8>,
}

#[derive(Serialize, Debug)]
pub enum PriorityLevel {
    None,     // 0th percentile
    Low,      // 25th percentile
    Medium,   // 50th percentile
    High,     // 75th percentile
    VeryHigh, // 95th percentile
    // labelled unsafe to prevent people using and draining their funds by accident
    UnsafeMax, // 100th percentile
    Default,   // 50th percentile
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetPriorityFeeEstimateResponse {
    pub priority_fee_estimate: Option<f64>,
    pub priority_fee_levels: Option<MicroLamportPriorityFeeLevels>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MicroLamportPriorityFeeLevels {
    pub none: f64,
    pub low: f64,
    pub medium: f64,
    pub high: f64,
    pub very_high: f64,
    pub unsafe_max: f64,
}

impl Helius {
    pub fn new(client: reqwest::Client, apikey: &str) -> Self {
        Self {
            client,
            mainnet_url: format!("https://mainnet.helius-rpc.com/?api-key={apikey}"),
            devnet_url: format!("https://devnet.helius-rpc.com/?api-key={apikey}"),
            id: AtomicU64::new(0),
        }
    }

    fn next_id(&self) -> String {
        self.id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_string()
    }

    pub fn get_url(&self, solana_net: &str) -> Result<&str, anyhow::Error> {
        match solana_net {
            "devnet" => Ok(&self.devnet_url),
            "mainnet" | "mainnet-beta" => Ok(&self.mainnet_url),
            _ => bail!("unknown solana_net: {}", solana_net),
        }
    }

    pub async fn get_priority_fee_estimate(
        &self,
        solana_net: &str,
        req: GetPriorityFeeEstimateRequest,
    ) -> Result<GetPriorityFeeEstimateResponse, anyhow::Error> {
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "getPriorityFeeEstimate",
            "params": [req],
        });

        #[derive(Deserialize)]
        struct HeliusResponse {
            result: GetPriorityFeeEstimateResponse,
        }

        let url = self.get_url(solana_net)?;
        let json = self
            .client
            .post(url)
            .json(&req)
            .send()
            .await?
            .error_for_status()?
            .json::<JsonValue>()
            .await?;
        let parsed = serde_json::from_value::<HeliusResponse>(json)?;
        Ok(parsed.result)
    }

    pub async fn get_assets_by_group(
        &self,
        solana_net: &str,
        collection: &str,
    ) -> Result<Vec<JsonValue>, anyhow::Error> {
        #[derive(Deserialize)]
        struct HeliusResult {
            items: Vec<JsonValue>,
            total: u64,
            // #[serde(flatten)]
            // extra: JsonValue,
        }

        #[derive(Deserialize)]
        struct HeliusResponse {
            result: HeliusResult,
        }

        const LIMIT: u64 = 1000;

        is_pubkey(collection)?;
        let url = self.get_url(solana_net)?;

        let mut page = 1;
        let mut assets = Vec::new();

        let mut req = serde_json::json!(
            {
                "jsonrpc": "2.0",
                "id": "",
                "method": "getAssetsByGroup",
                "params": {
                    "groupKey": "collection",
                    "groupValue": collection,
                    "page": 1,
                    "limit": LIMIT,
                    "sortBy": {
                        "sortBy": "created"
                    },
                    "displayOptions": {
                        "showUnverifiedCollections": false,
                        "showCollectionMetadata": false,
                        "showGrandTotal": false,
                        "showInscription": false,
                    }
                }
            }
        );

        loop {
            req["id"] = JsonValue::from(self.next_id());
            req["params"]["page"] = JsonValue::from(page);
            let resp = self
                .client
                .post(url)
                .json(&req)
                .send()
                .await?
                .error_for_status()?
                .json::<HeliusResponse>()
                .await?;

            assets.extend(resp.result.items);
            if resp.result.total < LIMIT {
                break;
            } else {
                page += 1;
            }
        }

        Ok(assets)
    }

    pub async fn get_asset(
        &self,
        solana_net: &str,
        mint_account: &str,
    ) -> Result<JsonValue, anyhow::Error> {
        #[derive(Deserialize)]
        struct HeliusResponse {
            result: JsonValue,
        }

        is_pubkey(mint_account)?;
        let url = self.get_url(solana_net)?;

        let req = serde_json::json!(
            {
                "jsonrpc": "2.0",
                "id": self.next_id(),
                "method": "getAsset",
                "params": {
                    "id": mint_account,
                }
            }
        );

        let resp = self
            .client
            .post(url)
            .json(&req)
            .send()
            .await?
            .error_for_status()?
            .json::<HeliusResponse>()
            .await?;

        Ok(resp.result)
    }
}
