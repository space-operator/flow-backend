//! Typed response structs for Jupiter API endpoints.
//!
//! These are used for deserialization testing and validation.
//! Node outputs remain `JsonValue` for maximum flexibility.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ========= Ultra Swap =========

#[derive(Debug, Serialize, Deserialize)]
pub struct Router {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UltraOrder {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    #[serde(default)]
    pub other_amount_threshold: Option<String>,
    #[serde(default)]
    pub slippage_bps: Option<u64>,
    #[serde(default)]
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(default)]
    pub transaction: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub router: Option<String>,
    #[serde(default)]
    pub gasless: Option<bool>,
    #[serde(default)]
    pub swap_usd_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShieldResponse {
    pub warnings: HashMap<String, Vec<ShieldWarning>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShieldWarning {
    #[serde(rename = "type")]
    pub warning_type: String,
    pub message: String,
    pub severity: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletHoldings {
    pub amount: String,
    pub ui_amount: f64,
    pub ui_amount_string: String,
    #[serde(default)]
    pub tokens: HashMap<String, Vec<TokenBalance>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalance {
    pub account: String,
    pub amount: String,
    pub ui_amount: f64,
    pub ui_amount_string: String,
    #[serde(default)]
    pub is_frozen: bool,
    #[serde(default)]
    pub is_associated_token_account: bool,
    pub decimals: u8,
    #[serde(default)]
    pub program_id: Option<String>,
}

// ========= Metis Swap =========

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapQuote {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    #[serde(default)]
    pub other_amount_threshold: Option<String>,
    #[serde(default)]
    pub swap_mode: Option<String>,
    #[serde(default)]
    pub slippage_bps: Option<u64>,
    #[serde(default)]
    pub platform_fee: Option<serde_json::Value>,
    #[serde(default)]
    pub price_impact_pct: Option<String>,
    #[serde(default)]
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(default)]
    pub context_slot: Option<u64>,
    #[serde(default)]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStep {
    pub swap_info: SwapInfo,
    pub percent: u64,
    #[serde(default)]
    pub bps: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    #[serde(default)]
    pub fee_amount: Option<String>,
    #[serde(default)]
    pub fee_mint: Option<String>,
}

// ========= Price =========

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceData {
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub liquidity: Option<f64>,
    #[serde(default)]
    pub usd_price: Option<f64>,
    #[serde(default)]
    pub block_id: Option<u64>,
    #[serde(default)]
    pub decimals: Option<u32>,
    #[serde(default)]
    pub price_change24h: Option<f64>,
}

// ========= Tokens V2 =========

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MintInformation {
    pub id: String,
    pub name: String,
    pub symbol: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub decimals: u8,
    #[serde(default)]
    pub usd_price: Option<f64>,
    #[serde(default)]
    pub mcap: Option<f64>,
    #[serde(default)]
    pub fdv: Option<f64>,
    #[serde(default)]
    pub liquidity: Option<f64>,
    #[serde(default)]
    pub total_supply: Option<f64>,
    #[serde(default)]
    pub circ_supply: Option<f64>,
    #[serde(default)]
    pub holder_count: Option<u64>,
    #[serde(default)]
    pub token_program: Option<String>,
    #[serde(default)]
    pub organic_score: Option<f64>,
    #[serde(default)]
    pub organic_score_label: Option<String>,
    #[serde(default)]
    pub is_verified: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub audit: Option<serde_json::Value>,
    #[serde(default)]
    pub first_pool: Option<serde_json::Value>,
    #[serde(default)]
    pub stats5m: Option<serde_json::Value>,
    #[serde(default)]
    pub stats1h: Option<serde_json::Value>,
    #[serde(default)]
    pub stats6h: Option<serde_json::Value>,
    #[serde(default)]
    pub stats24h: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenContent {
    pub data: Vec<TokenContentItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenContentItem {
    pub mint: String,
    #[serde(default)]
    pub contents: Vec<serde_json::Value>,
    #[serde(default)]
    pub token_summary: Option<serde_json::Value>,
    #[serde(default)]
    pub news_summary: Option<serde_json::Value>,
}

// ========= Earn / Lend =========

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EarnToken {
    pub id: u64,
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    pub asset_address: String,
    #[serde(default)]
    pub asset: Option<serde_json::Value>,
    #[serde(default)]
    pub total_assets: Option<String>,
    #[serde(default)]
    pub total_supply: Option<String>,
    #[serde(default)]
    pub convert_to_shares: Option<String>,
    #[serde(default)]
    pub convert_to_assets: Option<String>,
    #[serde(default)]
    pub rewards_rate: Option<String>,
    #[serde(default)]
    pub supply_rate: Option<String>,
    #[serde(default)]
    pub total_rate: Option<String>,
    #[serde(default)]
    pub liquidity_supply_data: Option<serde_json::Value>,
}

// ========= Portfolio =========

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioPositions {
    pub date: u64,
    pub owner: String,
    #[serde(default)]
    pub fetcher_reports: Vec<serde_json::Value>,
    #[serde(default)]
    pub elements: Vec<serde_json::Value>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub token_info: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioPlatform {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub defi_llama_id: Option<String>,
    #[serde(default)]
    pub is_deprecated: Option<bool>,
    #[serde(default)]
    pub tokens: Option<serde_json::Value>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub links: Option<serde_json::Value>,
}

// ========= Prediction =========

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictionEventList {
    pub data: Vec<serde_json::Value>,
    #[serde(default)]
    pub pagination: Option<PredictionPagination>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionPagination {
    #[serde(default)]
    pub start: Option<u64>,
    #[serde(default)]
    pub end: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub has_next: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictionLeaderboard {
    pub data: Vec<LeaderboardEntry>,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub owner_pubkey: String,
    #[serde(default)]
    pub realized_pnl_usd: Option<String>,
    #[serde(default)]
    pub total_volume_usd: Option<String>,
    #[serde(default)]
    pub predictions_count: Option<u64>,
    #[serde(default)]
    pub correct_predictions: Option<u64>,
    #[serde(default)]
    pub wrong_predictions: Option<u64>,
    #[serde(default)]
    pub win_rate_pct: Option<String>,
    #[serde(default)]
    pub period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictionTradeList {
    pub data: Vec<PredictionTrade>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionTrade {
    pub id: u64,
    pub owner_pubkey: String,
    pub market_id: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub event_title: Option<String>,
    #[serde(default)]
    pub market_title: Option<String>,
    #[serde(default)]
    pub amount_usd: Option<String>,
    #[serde(default)]
    pub price_usd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingStatus {
    pub trading_active: bool,
}

// ========= Trigger / Recurring =========

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOrderList {
    #[serde(default)]
    pub orders: Vec<serde_json::Value>,
    #[serde(default)]
    pub total_pages: Option<u64>,
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub total_items: Option<u64>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub order_status: Option<String>,
}

// ========= Tests =========

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        std::fs::read_to_string(
            format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name),
        )
        .unwrap()
    }

    #[test]
    fn test_deser_ultra_routers() {
        let json = read_fixture("ultra_routers.json");
        let parsed: Vec<Router> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
        assert!(!parsed[0].id.is_empty());
    }

    #[test]
    fn test_deser_ultra_shield() {
        let json = read_fixture("ultra_shield.json");
        let _parsed: ShieldResponse = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_deser_ultra_holdings() {
        let json = read_fixture("ultra_holdings.json");
        let parsed: WalletHoldings = serde_json::from_str(&json).unwrap();
        assert!(!parsed.amount.is_empty());
    }

    #[test]
    fn test_deser_swap_quote() {
        let json = read_fixture("swap_quote.json");
        let parsed: SwapQuote = serde_json::from_str(&json).unwrap();
        assert!(!parsed.input_mint.is_empty());
        assert!(!parsed.route_plan.is_empty());
    }

    #[test]
    fn test_deser_price() {
        let json = read_fixture("price.json");
        let parsed: HashMap<String, PriceData> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
    }

    #[test]
    fn test_deser_tokens_search() {
        let json = read_fixture("tokens_search.json");
        let parsed: Vec<MintInformation> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
        assert!(!parsed[0].id.is_empty());
    }

    #[test]
    fn test_deser_tokens_recent() {
        let json = read_fixture("tokens_recent.json");
        let _parsed: Vec<MintInformation> = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_deser_tokens_category() {
        let json = read_fixture("tokens_category.json");
        let _parsed: Vec<MintInformation> = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_deser_tokens_tag() {
        let json = read_fixture("tokens_tag.json");
        let _parsed: Vec<MintInformation> = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_deser_tokens_content_cooking() {
        let json = read_fixture("tokens_content_cooking.json");
        let _parsed: TokenContent = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_deser_earn_tokens() {
        let json = read_fixture("earn_tokens.json");
        let parsed: Vec<EarnToken> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
        assert!(!parsed[0].address.is_empty());
    }

    #[test]
    fn test_deser_portfolio_positions() {
        let json = read_fixture("portfolio_positions.json");
        let parsed: PortfolioPositions = serde_json::from_str(&json).unwrap();
        assert!(!parsed.owner.is_empty());
    }

    #[test]
    fn test_deser_portfolio_platforms() {
        let json = read_fixture("portfolio_platforms.json");
        let parsed: Vec<PortfolioPlatform> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
    }

    #[test]
    fn test_deser_program_id_to_label() {
        let json = read_fixture("program_id_to_label.json");
        let parsed: HashMap<String, String> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
    }

    #[test]
    fn test_deser_prediction_events() {
        let json = read_fixture("prediction_events.json");
        let parsed: PredictionEventList = serde_json::from_str(&json).unwrap();
        assert!(!parsed.data.is_empty());
    }

    #[test]
    fn test_deser_prediction_leaderboards() {
        let json = read_fixture("prediction_leaderboards.json");
        let parsed: PredictionLeaderboard = serde_json::from_str(&json).unwrap();
        assert!(!parsed.data.is_empty());
    }

    #[test]
    fn test_deser_prediction_trades() {
        let json = read_fixture("prediction_trades.json");
        let parsed: PredictionTradeList = serde_json::from_str(&json).unwrap();
        assert!(!parsed.data.is_empty());
    }

    #[test]
    fn test_deser_trading_status() {
        let json = read_fixture("prediction_trading_status.json");
        let _parsed: TradingStatus = serde_json::from_str(&json).unwrap();
    }
}
