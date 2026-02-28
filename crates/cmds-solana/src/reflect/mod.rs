pub mod helper;

// Health
pub mod reflect_health_check;

// Stablecoin
pub mod reflect_get_stablecoin_types;
pub mod reflect_get_all_exchange_rates;
pub mod reflect_get_exchange_rate;
pub mod reflect_get_historical_exchange_rates;
pub mod reflect_get_all_apy;
pub mod reflect_get_apy;
pub mod reflect_get_historical_apy;
pub mod reflect_mint_stablecoin;
pub mod reflect_burn_stablecoin;
pub mod reflect_get_mint_burn_quote;
pub mod reflect_get_mint_burn_limits;

// Integration - Setup
pub mod reflect_initialize_integration;
pub mod reflect_initialize_flow;
pub mod reflect_initialize_stablecoin;
pub mod reflect_initialize_vault;
pub mod reflect_initialize_user_account;

// Integration - Config & Info
pub mod reflect_get_integration_config;
pub mod reflect_update_integration_config;
pub mod reflect_upload_integration_logo;
pub mod reflect_list_integrations_by_authority;
pub mod reflect_get_verified_integrations;
pub mod reflect_check_integration_status;
pub mod reflect_get_integration_exchange_rate;

// Integration - Quotes
pub mod reflect_get_integration_quote;
pub mod reflect_get_integration_quote_with_fees;

// Integration - Operations
pub mod reflect_mint_integration;
pub mod reflect_redeem_integration;
pub mod reflect_flow_mint;
pub mod reflect_flow_redeem;
pub mod reflect_claim_integration;

// Integration - Admin
pub mod reflect_reveal_api_key;
pub mod reflect_rotate_api_key;
pub mod reflect_transfer_authority;
pub mod reflect_whitelist_address;

// Integration - Stats & Events
pub mod reflect_get_integration_events;
pub mod reflect_get_integration_stats;
pub mod reflect_get_integration_historical_stats;

// Protocol Stats
pub mod reflect_get_protocol_stats;
pub mod reflect_get_historical_stats;

// Events
pub mod reflect_get_recent_events;
pub mod reflect_get_events_by_signer;
