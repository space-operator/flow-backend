//! Typed response structs for Bridge.xyz API endpoints.
//!
//! These structs are for testing and validation only — node outputs remain `JsonValue`.
//! All structs derived from the Bridge API docs at <https://apidocs.bridge.xyz>.

use serde::{Deserialize, Serialize};

// ─── Pagination wrapper ──────────────────────────────────────────────────────

/// Standard paginated list response used by most Bridge list endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedList<T> {
    pub count: Option<u64>,
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination_token: Option<String>,
}

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub source: Option<BridgeErrorSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeErrorSource {
    pub location: String,
    pub key: String,
}

// ─── Exchange Rates ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub midmarket_rate: String,
    pub buy_rate: String,
    pub sell_rate: String,
}

// ─── Customers ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub status: String,
    #[serde(rename = "type")]
    pub customer_type: String,
    #[serde(default)]
    pub persona_inquiry_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub rejection_reasons: Vec<String>,
    #[serde(default)]
    pub has_accepted_terms_of_service: bool,
    #[serde(default)]
    pub endorsements: Vec<Endorsement>,
    #[serde(default)]
    pub future_requirements_due: Vec<String>,
    #[serde(default)]
    pub requirements_due: Vec<String>,
    #[serde(default)]
    pub capabilities: Option<CustomerCapabilities>,
    #[serde(default)]
    pub associated_persons: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomerCapabilities {
    #[serde(default)]
    pub payin_crypto: Option<String>,
    #[serde(default)]
    pub payout_crypto: Option<String>,
    #[serde(default)]
    pub payin_fiat: Option<String>,
    #[serde(default)]
    pub payout_fiat: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Endorsement {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub requirements: Option<EndorsementRequirements>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndorsementRequirements {
    #[serde(default)]
    pub complete: Vec<String>,
    #[serde(default)]
    pub pending: Vec<String>,
    #[serde(default)]
    pub missing: Option<serde_json::Value>,
    #[serde(default)]
    pub issues: Vec<String>,
}

// ─── KYC Links ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct KycLink {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(rename = "type", default)]
    pub link_type: Option<String>,
    #[serde(default)]
    pub kyc_link: Option<String>,
    #[serde(default)]
    pub tos_link: Option<String>,
    #[serde(default)]
    pub kyc_status: Option<String>,
    #[serde(default)]
    pub rejection_reasons: Vec<String>,
    #[serde(default)]
    pub tos_status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub persona_inquiry_type: Option<String>,
}

// ─── External Accounts ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalAccount {
    pub id: String,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub account_owner_name: Option<String>,
    #[serde(default)]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub account_type: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub beneficiary_address_valid: Option<bool>,
    #[serde(default)]
    pub account: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Transfers ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    #[serde(default)]
    pub client_reference_id: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub on_behalf_of: Option<String>,
    #[serde(default)]
    pub developer_fee: Option<String>,
    #[serde(default)]
    pub developer_fee_percent: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub source: Option<serde_json::Value>,
    #[serde(default)]
    pub destination: Option<serde_json::Value>,
    #[serde(default)]
    pub source_deposit_instructions: Option<serde_json::Value>,
    #[serde(default)]
    pub receipt: Option<TransferReceipt>,
    #[serde(default)]
    pub return_details: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferReceipt {
    #[serde(default)]
    pub initial_amount: Option<String>,
    #[serde(default)]
    pub developer_fee: Option<String>,
    #[serde(default)]
    pub exchange_fee: Option<String>,
    #[serde(default)]
    pub subtotal_amount: Option<String>,
    #[serde(default)]
    pub gas_fee: Option<String>,
    #[serde(default)]
    pub final_amount: Option<String>,
    #[serde(default)]
    pub source_tx_hash: Option<String>,
    #[serde(default)]
    pub destination_tx_hash: Option<String>,
    #[serde(default)]
    pub exchange_rate: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Dry-run transfer route validation response.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRoute {
    pub state: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

// ─── Liquidation Addresses ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct LiquidationAddress {
    pub id: String,
    #[serde(default)]
    pub chain: Option<String>,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub external_account_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Virtual Accounts ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualAccount {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub developer_fee_percent: Option<String>,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub source_deposit_instructions: Option<serde_json::Value>,
    #[serde(default)]
    pub destination: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<String>,
}

// ─── Wallets ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub id: String,
    #[serde(default)]
    pub chain: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Webhooks ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub event_categories: Vec<String>,
}

// ─── Card Accounts ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CardAccount {
    pub id: String,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub client_reference_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub status_reason: Option<String>,
    #[serde(default)]
    pub cardholder_name: Option<serde_json::Value>,
    #[serde(default)]
    pub card_details: Option<serde_json::Value>,
    #[serde(default)]
    pub balances: Option<serde_json::Value>,
    #[serde(default)]
    pub crypto_account: Option<serde_json::Value>,
    #[serde(default)]
    pub funding_instructions: Option<serde_json::Value>,
}

// ─── Static Memos ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct StaticMemo {
    pub id: String,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub chain: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Prefunded Accounts ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct PrefundedAccount {
    pub id: String,
    #[serde(default)]
    pub balance: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Reference Data ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub name: String,
    pub alpha3: String,
    #[serde(default)]
    pub postal_code_format: Option<String>,
    #[serde(default)]
    pub subdivisions: Vec<Subdivision>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subdivision {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OccupationCode {
    pub display_name: String,
    pub code: String,
}

// ─── Developer Fees ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct DeveloperFees {
    #[serde(default)]
    pub default_liquidation_address_fee_percent: Option<String>,
}

// ─── Crypto Return Policies ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoReturnPolicy {
    pub id: String,
    #[serde(default)]
    pub chain: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub return_address: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Associated Persons ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AssociatedPerson {
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── Rewards ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct RewardRate {
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub effective_date: Option<String>,
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {name}: {e}"))
    }

    fn sandbox_key() -> Option<String> {
        std::env::var("BRIDGE_API_KEY").ok()
    }

    const SANDBOX_BASE: &str = "https://api.sandbox.bridge.xyz/v0";

    // ── Deserialization tests ────────────────────────────────────────────

    #[test]
    fn test_deserialize_exchange_rate() {
        let json = fixture("exchange_rate.json");
        let rate: ExchangeRate = serde_json::from_str(&json).unwrap();
        assert!(!rate.midmarket_rate.is_empty());
        assert!(!rate.buy_rate.is_empty());
        assert!(!rate.sell_rate.is_empty());
    }

    #[test]
    fn test_deserialize_customer() {
        let json = fixture("customer.json");
        let c: Customer = serde_json::from_str(&json).unwrap();
        assert!(!c.id.is_empty());
        assert_eq!(c.status, "active");
        assert_eq!(c.customer_type, "business");
    }

    #[test]
    fn test_deserialize_customer_list() {
        let json = fixture("customer_list.json");
        let list: PaginatedList<Customer> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(1));
        assert!(!list.data.is_empty());
        assert!(!list.data[0].id.is_empty());
    }

    #[test]
    fn test_deserialize_kyc_link_list() {
        let json = fixture("kyc_link_list.json");
        let list: PaginatedList<KycLink> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(1));
        assert!(!list.data.is_empty());
    }

    #[test]
    fn test_deserialize_transfer_list() {
        let json = fixture("transfer_list.json");
        let list: PaginatedList<Transfer> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(0));
        assert!(list.data.is_empty());
    }

    #[test]
    fn test_deserialize_external_account_list() {
        let json = fixture("external_account_list.json");
        let list: PaginatedList<ExternalAccount> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(0));
    }

    #[test]
    fn test_deserialize_liquidation_address_list() {
        let json = fixture("liquidation_address_list.json");
        let list: PaginatedList<LiquidationAddress> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(0));
    }

    #[test]
    fn test_deserialize_virtual_account_list() {
        let json = fixture("virtual_account_list.json");
        let list: PaginatedList<VirtualAccount> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(0));
    }

    #[test]
    fn test_deserialize_countries() {
        let json = fixture("countries.json");
        let list: PaginatedList<Country> = serde_json::from_str(&json).unwrap();
        assert!(!list.data.is_empty());
        assert!(!list.data[0].name.is_empty());
        assert!(!list.data[0].alpha3.is_empty());
    }

    #[test]
    fn test_deserialize_occupation_codes() {
        let json = fixture("occupation_codes.json");
        let codes: Vec<OccupationCode> = serde_json::from_str(&json).unwrap();
        assert!(!codes.is_empty());
        assert!(!codes[0].display_name.is_empty());
        assert!(!codes[0].code.is_empty());
    }

    #[test]
    fn test_deserialize_fees() {
        let json = fixture("fees.json");
        let fees: DeveloperFees = serde_json::from_str(&json).unwrap();
        assert!(fees.default_liquidation_address_fee_percent.is_some());
    }

    #[test]
    fn test_deserialize_error() {
        let json = fixture("error.json");
        let err: BridgeError = serde_json::from_str(&json).unwrap();
        assert!(!err.code.is_empty());
        assert!(!err.message.is_empty());
        assert!(err.source.is_some());
    }

    #[test]
    fn test_deserialize_static_memo_list() {
        let json = fixture("static_memo_list.json");
        let list: PaginatedList<StaticMemo> = serde_json::from_str(&json).unwrap();
        assert_eq!(list.count, Some(0));
    }

    #[test]
    fn test_deserialize_prefunded_account_list() {
        let json = fixture("prefunded_account_list.json");
        // Prefunded accounts don't have count field
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["data"].is_array());
    }

    #[test]
    fn test_deserialize_crypto_return_policy_list() {
        let json = fixture("crypto_return_policy_list.json");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["data"].is_array());
    }

    // ── Sandbox integration tests ───────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn test_live_exchange_rate() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/exchange_rates"))
            .header("Api-Key", &api_key)
            .query(&[("from", "usd"), ("to", "eur")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let rate: ExchangeRate = resp.json().await.expect("json parse failed");
        assert!(!rate.midmarket_rate.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_customers() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/customers"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let list: PaginatedList<Customer> = resp.json().await.expect("json parse failed");
        assert!(list.count.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_customer() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        // First get the list to find a customer ID
        let resp = client
            .get(format!("{SANDBOX_BASE}/customers"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "1")])
            .send()
            .await
            .expect("request failed");
        let list: PaginatedList<Customer> = resp.json().await.expect("json parse failed");
        if list.data.is_empty() {
            eprintln!("No customers in sandbox, skipping");
            return;
        }
        let customer_id = &list.data[0].id;
        let resp = client
            .get(format!("{SANDBOX_BASE}/customers/{customer_id}"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let c: Customer = resp.json().await.expect("json parse failed");
        assert_eq!(c.id, *customer_id);
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_countries() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/lists/countries"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let list: PaginatedList<Country> = resp.json().await.expect("json parse failed");
        assert!(!list.data.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_occupation_codes() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/lists/occupation_codes"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let codes: Vec<OccupationCode> = resp.json().await.expect("json parse failed");
        assert!(!codes.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_transfers() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/transfers"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _list: PaginatedList<Transfer> = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_external_accounts() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/external_accounts"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _list: PaginatedList<ExternalAccount> = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_liquidation_addresses() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/liquidation_addresses"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _list: PaginatedList<LiquidationAddress> = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_kyc_links() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/kyc_links"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "2")])
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _list: PaginatedList<KycLink> = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_developer_fees() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/developer/fees"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _fees: DeveloperFees = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_static_memos() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/static_memos"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _list: PaginatedList<StaticMemo> = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_crypto_return_policies() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/crypto_return_policies"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_static_templates() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/transfers/static_templates"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_funds_requests() {
        let api_key = match sandbox_key() {
            Some(k) => k,
            None => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{SANDBOX_BASE}/funds_requests"))
            .header("Api-Key", &api_key)
            .send()
            .await
            .expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }
}
