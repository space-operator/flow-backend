//! DFlow API Nodes
//!
//! Space Operator nodes for DFlow Trading, Metadata, and Proof APIs.
//!
//! ## Trading API
//! - `dflow_get_order` - Unified swap order
//! - `dflow_get_quote` - Imperative swap quote **(DEPRECATED)**
//! - `dflow_create_swap` - Create swap transaction **(DEPRECATED)**
//! - `dflow_get_intent_quote` - Declarative intent quote
//! - `dflow_submit_intent` - Submit intent swap
//! - `dflow_init_prediction_market` - Initialize prediction market
//! - `dflow_get_tokens` - Supported tokens
//! - `dflow_get_tokens_with_decimals` - Tokens with decimals
//! - `dflow_get_venues` - Trading venues
//!
//! ## Metadata API — Events
//! - `dflow_get_event` - Single event
//! - `dflow_get_events` - List events
//! - `dflow_get_forecast_history` - Forecast history
//!
//! ## Metadata API — Markets
//! - `dflow_get_market` - Single market
//! - `dflow_get_markets` - List markets
//! - `dflow_get_market_by_mint` - Market by mint
//! - `dflow_get_markets_batch` - Batch market lookup
//! - `dflow_get_outcome_mints` - All outcome mints
//! - `dflow_filter_outcome_mints` - Filter outcome mints
//!
//! ## Metadata API — Orderbook
//! - `dflow_get_orderbook` - Orderbook by ticker
//! - `dflow_get_orderbook_by_mint` - Orderbook by mint
//!
//! ## Metadata API — Trades
//! - `dflow_get_trades` - Trade history
//! - `dflow_get_trades_by_mint` - Trades by mint
//!
//! ## Metadata API — Live Data
//! - `dflow_get_live_data` - Live data by milestone IDs
//! - `dflow_get_live_data_by_event` - Live data by event
//! - `dflow_get_live_data_by_mint` - Live data by mint
//!
//! ## Metadata API — Series
//! - `dflow_get_series` - List series
//! - `dflow_get_series_by_ticker` - Series by ticker
//!
//! ## Metadata API — Tags, Sports, Search
//! - `dflow_get_tags_by_categories` - Tags by categories
//! - `dflow_get_filters_by_sports` - Filters by sports
//! - `dflow_search_events` - Search events
//!
//! ## Proof API
//! - `dflow_verify_address` - Verify wallet KYC status

pub mod response_types;

pub mod dflow_get_order;
pub mod dflow_get_quote;
pub mod dflow_create_swap;
pub mod dflow_get_intent_quote;
pub mod dflow_submit_intent;
pub mod dflow_init_prediction_market;
pub mod dflow_get_tokens;
pub mod dflow_get_tokens_with_decimals;
pub mod dflow_get_venues;
pub mod dflow_get_event;
pub mod dflow_get_events;
pub mod dflow_get_forecast_history;
pub mod dflow_get_market;
pub mod dflow_get_markets;
pub mod dflow_get_market_by_mint;
pub mod dflow_get_markets_batch;
pub mod dflow_get_outcome_mints;
pub mod dflow_filter_outcome_mints;
pub mod dflow_get_orderbook;
pub mod dflow_get_orderbook_by_mint;
pub mod dflow_get_trades;
pub mod dflow_get_trades_by_mint;
pub mod dflow_get_live_data;
pub mod dflow_get_live_data_by_event;
pub mod dflow_get_live_data_by_mint;
pub mod dflow_get_series;
pub mod dflow_get_series_by_ticker;
pub mod dflow_get_tags_by_categories;
pub mod dflow_get_filters_by_sports;
pub mod dflow_search_events;
pub mod dflow_verify_address;

