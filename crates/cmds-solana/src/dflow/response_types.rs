//! Typed response structs for DFlow API endpoints.
//!
//! These structs document the expected response shapes from DFlow's Trading,
//! Metadata, and Proof APIs. They are used for deserialization testing and
//! optional typed access. Node outputs remain `JsonValue` for compatibility.
//!
//! Field naming follows the API's JSON conventions (camelCase for most fields,
//! snake_case for some Metadata API pagination fields).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Metadata API — Markets
// =============================================================================

/// On-chain account information for a prediction market, keyed by settlement mint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketAccountInfo {
    #[serde(rename = "marketLedger")]
    pub market_ledger: String,
    #[serde(rename = "yesMint")]
    pub yes_mint: String,
    #[serde(rename = "noMint")]
    pub no_mint: String,
    #[serde(rename = "isInitialized")]
    pub is_initialized: bool,
    #[serde(rename = "redemptionStatus")]
    pub redemption_status: Option<String>,
    #[serde(rename = "scalarOutcomePct", default)]
    pub scalar_outcome_pct: Option<i64>,
}

/// A prediction market.
///
/// Returned by `GET /api/v1/market/{id}`, nested in events, and in market lists.
/// The `accounts` field is a map keyed by settlement mint address (USDC or CASH).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Market {
    pub ticker: String,
    #[serde(rename = "eventTicker")]
    pub event_ticker: String,
    #[serde(rename = "marketType")]
    pub market_type: String,
    pub title: String,
    pub subtitle: String,
    #[serde(rename = "yesSubTitle")]
    pub yes_sub_title: String,
    #[serde(rename = "noSubTitle")]
    pub no_sub_title: String,
    #[serde(rename = "openTime")]
    pub open_time: i64,
    #[serde(rename = "closeTime")]
    pub close_time: i64,
    #[serde(rename = "expirationTime")]
    pub expiration_time: i64,
    pub status: String,
    pub volume: i64,
    pub result: String,
    #[serde(rename = "openInterest")]
    pub open_interest: i64,
    #[serde(rename = "canCloseEarly")]
    pub can_close_early: bool,
    #[serde(rename = "rulesPrimary")]
    pub rules_primary: String,
    /// Map of settlement mint address → account info.
    pub accounts: HashMap<String, MarketAccountInfo>,
    // Optional fields
    #[serde(rename = "rulesSecondary", default)]
    pub rules_secondary: Option<String>,
    #[serde(rename = "earlyCloseCondition", default)]
    pub early_close_condition: Option<String>,
    #[serde(rename = "yesBid", default)]
    pub yes_bid: Option<String>,
    #[serde(rename = "yesAsk", default)]
    pub yes_ask: Option<String>,
    #[serde(rename = "noBid", default)]
    pub no_bid: Option<String>,
    #[serde(rename = "noAsk", default)]
    pub no_ask: Option<String>,
}

/// Paginated market list response from `GET /api/v1/markets`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketListResponse {
    pub markets: Vec<Market>,
    pub cursor: Option<serde_json::Value>,
}

// =============================================================================
// Metadata API — Events
// =============================================================================

/// Settlement source reference (e.g., AP, ESPN).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettlementSource {
    pub name: String,
    pub url: String,
}

/// A prediction market event.
///
/// Returned by `GET /api/v1/event/{id}` and in event lists.
/// The `markets` field is only present when `withNestedMarkets=true`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub ticker: String,
    #[serde(rename = "seriesTicker")]
    pub series_ticker: String,
    pub title: String,
    pub subtitle: String,
    // Optional fields
    #[serde(default)]
    pub competition: Option<String>,
    #[serde(rename = "competitionScope", default)]
    pub competition_scope: Option<String>,
    #[serde(rename = "imageUrl", default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub liquidity: Option<u64>,
    #[serde(rename = "openInterest", default)]
    pub open_interest: Option<u64>,
    #[serde(default)]
    pub volume: Option<u64>,
    #[serde(default)]
    pub volume24h: Option<u64>,
    #[serde(rename = "strikeDate", default)]
    pub strike_date: Option<i64>,
    #[serde(rename = "strikePeriod", default)]
    pub strike_period: Option<String>,
    #[serde(rename = "settlementSources", default)]
    pub settlement_sources: Option<Vec<SettlementSource>>,
    #[serde(default)]
    pub markets: Option<Vec<Market>>,
}

/// Paginated event list response from `GET /api/v1/events`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventListResponse {
    pub events: Vec<Event>,
    pub cursor: Option<serde_json::Value>,
}

// =============================================================================
// Metadata API — Orderbook
// =============================================================================

/// Orderbook depth for a prediction market.
///
/// `yes_bids` and `no_bids` are maps of price (4-decimal string like "0.0500")
/// to quantity (integer contract count).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Orderbook {
    pub yes_bids: HashMap<String, i64>,
    pub no_bids: HashMap<String, i64>,
    pub sequence: i64,
}

// =============================================================================
// Metadata API — Trades
// =============================================================================

/// A single trade record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trade {
    #[serde(rename = "tradeId")]
    pub trade_id: String,
    pub ticker: String,
    pub price: i32,
    pub count: i32,
    #[serde(rename = "yesPrice")]
    pub yes_price: i32,
    #[serde(rename = "noPrice")]
    pub no_price: i32,
    #[serde(rename = "yesPriceDollars")]
    pub yes_price_dollars: String,
    #[serde(rename = "noPriceDollars")]
    pub no_price_dollars: String,
    #[serde(rename = "takerSide")]
    pub taker_side: String,
    #[serde(rename = "createdTime")]
    pub created_time: i64,
}

/// Paginated trade list response from `GET /api/v1/trades`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradeListResponse {
    pub trades: Vec<Trade>,
    pub cursor: Option<String>,
}

// =============================================================================
// Metadata API — Series
// =============================================================================

/// Settlement source for a series.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeriesSettlementSource {
    pub name: String,
    pub url: String,
}

/// A prediction market series template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Series {
    pub ticker: String,
    pub frequency: String,
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    #[serde(rename = "settlementSources")]
    pub settlement_sources: Vec<SeriesSettlementSource>,
    #[serde(rename = "contractUrl")]
    pub contract_url: String,
    #[serde(rename = "contractTermsUrl")]
    pub contract_terms_url: String,
    #[serde(rename = "productMetadata")]
    pub product_metadata: serde_json::Value,
    #[serde(rename = "feeType")]
    pub fee_type: String,
    #[serde(rename = "feeMultiplier")]
    pub fee_multiplier: f64,
    #[serde(rename = "additionalProhibitions")]
    pub additional_prohibitions: Vec<String>,
}

/// Series list response from `GET /api/v1/series`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeriesListResponse {
    pub series: Vec<Series>,
}

// =============================================================================
// Trading API — Order
// =============================================================================

/// Response from `GET /order` (unified swap endpoint).
///
/// Replaces the deprecated `/quote` and `/swap` endpoints.
/// Contains route plan, pricing, and optionally a transaction to sign.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "minOutAmount")]
    pub min_out_amount: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: i32,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "contextSlot")]
    pub context_slot: i64,
    #[serde(rename = "executionMode")]
    pub execution_mode: String,
    // Optional fields (present when userPublicKey is provided)
    #[serde(rename = "computeUnitLimit", default)]
    pub compute_unit_limit: Option<i32>,
    #[serde(default)]
    pub transaction: Option<String>,
    #[serde(rename = "lastValidBlockHeight", default)]
    pub last_valid_block_height: Option<i64>,
    #[serde(rename = "prioritizationFeeLamports", default)]
    pub prioritization_fee_lamports: Option<i64>,
    #[serde(rename = "revertMint", default)]
    pub revert_mint: Option<String>,
    #[serde(rename = "routePlan", default)]
    pub route_plan: Option<serde_json::Value>,
    #[serde(rename = "platformFee", default)]
    pub platform_fee: Option<serde_json::Value>,
    #[serde(rename = "prioritizationType", default)]
    pub prioritization_type: Option<serde_json::Value>,
    #[serde(rename = "predictionMarketSlippageBps", default)]
    pub prediction_market_slippage_bps: Option<i32>,
    #[serde(rename = "initPredictionMarketCost", default)]
    pub init_prediction_market_cost: Option<i32>,
    #[serde(rename = "predictionMarketInitPayerMustSign", default)]
    pub prediction_market_init_payer_must_sign: Option<bool>,
}

// =============================================================================
// Proof API
// =============================================================================

/// KYC verification response from `GET /verify/{address}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifyResponse {
    pub verified: bool,
}

// =============================================================================
// Outcome Mints
// =============================================================================

/// Response from `GET /api/v1/outcome_mints`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeMintListResponse {
    pub mints: Vec<String>,
}

// =============================================================================
// DFlow API Error
// =============================================================================

/// Standard error response shared across all DFlow APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DFlowError {
    pub code: String,
    pub msg: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_market() {
        let json_str = include_str!("fixtures/market.json");
        let market: Market =
            serde_json::from_str(json_str).expect("Failed to deserialize market fixture");
        assert_eq!(market.ticker, "PRES-2024-KH");
        assert_eq!(market.market_type, "binary");
        assert_eq!(market.status, "finalized");
        assert!(market.accounts.contains_key("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        assert!(market.accounts.contains_key("CASHx9KJUStyftLFWGvEVf59SGeG9sh5FfcnZMVPCASH"));
        let usdc_account = &market.accounts["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"];
        assert!(!usdc_account.yes_mint.is_empty());
        assert!(!usdc_account.no_mint.is_empty());
    }

    #[test]
    fn test_deserialize_market_list() {
        let json_str = include_str!("fixtures/markets_list.json");
        let resp: MarketListResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize market list fixture");
        assert!(!resp.markets.is_empty());
        assert!(resp.cursor.is_some());
    }

    #[test]
    fn test_deserialize_event() {
        let json_str = include_str!("fixtures/event.json");
        let event: Event =
            serde_json::from_str(json_str).expect("Failed to deserialize event fixture");
        assert_eq!(event.ticker, "KXSB-26");
        assert_eq!(event.series_ticker, "KXSB");
        assert!(event.settlement_sources.is_some());
        let sources = event.settlement_sources.as_ref().unwrap();
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_deserialize_event_list() {
        let json_str = include_str!("fixtures/events_list.json");
        let resp: EventListResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize event list fixture");
        assert!(!resp.events.is_empty());
    }

    #[test]
    fn test_deserialize_orderbook() {
        let json_str = include_str!("fixtures/orderbook.json");
        let ob: Orderbook =
            serde_json::from_str(json_str).expect("Failed to deserialize orderbook fixture");
        assert!(ob.sequence > 0);
        assert!(!ob.yes_bids.is_empty() || !ob.no_bids.is_empty());
    }

    #[test]
    fn test_deserialize_trades() {
        let json_str = include_str!("fixtures/trades.json");
        let resp: TradeListResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize trades fixture");
        assert!(!resp.trades.is_empty());
        let trade = &resp.trades[0];
        assert!(!trade.trade_id.is_empty());
        assert!(!trade.ticker.is_empty());
        assert!(trade.price >= 0);
        assert!(trade.created_time > 0);
    }

    #[test]
    fn test_deserialize_series() {
        let json_str = include_str!("fixtures/series.json");
        let series: Series =
            serde_json::from_str(json_str).expect("Failed to deserialize series fixture");
        assert_eq!(series.ticker, "KXSB");
        assert_eq!(series.category, "Sports");
        assert!(!series.settlement_sources.is_empty());
    }

    #[test]
    fn test_deserialize_series_list() {
        let json_str = include_str!("fixtures/series_list.json");
        let resp: SeriesListResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize series list fixture");
        assert!(!resp.series.is_empty());
    }

    #[test]
    fn test_deserialize_verify() {
        let json_str = include_str!("fixtures/verify.json");
        let resp: VerifyResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize verify fixture");
        assert!(!resp.verified);
    }

    #[test]
    fn test_deserialize_outcome_mints() {
        let json_str = include_str!("fixtures/outcome_mints.json");
        let resp: OutcomeMintListResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize outcome mints fixture");
        assert!(!resp.mints.is_empty());
    }

    #[test]
    fn test_deserialize_search_events() {
        let json_str = include_str!("fixtures/search.json");
        let resp: EventListResponse =
            serde_json::from_str(json_str).expect("Failed to deserialize search fixture");
        assert!(!resp.events.is_empty());
    }

    #[test]
    fn test_deserialize_error() {
        let json_str = r#"{"code": "404", "msg": "Not found"}"#;
        let err: DFlowError = serde_json::from_str(json_str).expect("Failed to deserialize error");
        assert_eq!(err.code, "404");
    }

    // =========================================================================
    // Integration tests — hit live dev endpoints
    // =========================================================================

    #[tokio::test]
    #[ignore]
    async fn test_live_get_markets() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/markets")
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: MarketListResponse = resp.json().await.expect("typed deser failed");
        assert!(!body.markets.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_events() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/events")
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: EventListResponse = resp.json().await.expect("typed deser failed");
        assert!(!body.events.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_trades() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/trades")
            .query(&[("limit", "3")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: TradeListResponse = resp.json().await.expect("typed deser failed");
        assert!(!body.trades.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_series() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/series")
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: SeriesListResponse = resp.json().await.expect("typed deser failed");
        assert!(!body.series.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_orderbook() {
        // First get an active market to query its orderbook
        let client = reqwest::Client::new();
        let markets_resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/markets")
            .query(&[("limit", "1"), ("status", "active")])
            .send()
            .await
            .expect("markets request failed");
        let markets: MarketListResponse = markets_resp.json().await.expect("markets deser");
        if markets.markets.is_empty() {
            eprintln!("No active markets found, skipping orderbook test");
            return;
        }
        let ticker = &markets.markets[0].ticker;

        let resp = client
            .get(format!(
                "https://dev-prediction-markets-api.dflow.net/api/v1/orderbook/{ticker}"
            ))
            .send()
            .await
            .expect("orderbook request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _ob: Orderbook = resp.json().await.expect("orderbook typed deser failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_search_events() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/search")
            .query(&[("q", "bitcoin"), ("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: EventListResponse = resp.json().await.expect("typed deser failed");
        // Search may return empty results, that's ok
        assert!(body.events.len() <= 2);
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_tags_by_categories() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/tags_by_categories")
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_filters_by_sports() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/filters_by_sports")
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_outcome_mints() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-prediction-markets-api.dflow.net/api/v1/outcome_mints")
            .query(&[("limit", "5")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: OutcomeMintListResponse = resp.json().await.expect("typed deser failed");
        assert!(!body.mints.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_verify_address() {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://proof.dflow.net/verify/11111111111111111111111111111112")
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: VerifyResponse = resp.json().await.expect("typed deser failed");
        assert!(!body.verified); // System program address is not KYC'd
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_trading_get_tokens() {
        let api_key = match std::env::var("DFLOW_API_KEY") {
            Ok(k) => k,
            Err(_) => {
                eprintln!("DFLOW_API_KEY not set, skipping trading API test");
                return;
            }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-quote-api.dflow.net/tokens")
            .header("x-api-key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: serde_json::Value = resp.json().await.expect("json parse failed");
        assert!(body.is_array());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_trading_get_venues() {
        let api_key = match std::env::var("DFLOW_API_KEY") {
            Ok(k) => k,
            Err(_) => {
                eprintln!("DFLOW_API_KEY not set, skipping trading API test");
                return;
            }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get("https://dev-quote-api.dflow.net/venues")
            .header("x-api-key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let body: serde_json::Value = resp.json().await.expect("json parse failed");
        assert!(body.is_array());
    }
}
