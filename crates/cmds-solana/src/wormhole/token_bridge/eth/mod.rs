use std::num::ParseIntError;

use serde::{Deserialize, Serialize};

use super::Address;

pub mod attest_from_eth;
pub mod create_wrapped_on_eth;
pub mod redeem_on_eth;
pub mod transfer_from_eth;

#[derive(Serialize, Deserialize, Debug)]
struct GasUsed {
    #[serde(rename = "type")]
    gas_type: String,
    hex: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EffectiveGasPrice {
    #[serde(rename = "type")]
    gas_type: String,
    hex: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Receipt {
    to: String,
    from: String,
    contract_address: Option<String>,
    #[serde(rename = "transactionIndex")]
    transaction_index: u32,
    #[serde(rename = "gasUsed")]
    gas_used: GasUsed,
    #[serde(rename = "logsBloom")]
    logs_bloom: String,
    #[serde(rename = "blockHash")]
    block_hash: String,
    #[serde(rename = "transactionHash")]
    transaction_hash: String,
    logs: Vec<Log>,
    #[serde(rename = "blockNumber")]
    block_number: u32,
    confirmations: u32,
    #[serde(rename = "cumulativeGasUsed")]
    cumulative_gas_used: GasUsed,
    #[serde(rename = "effectiveGasPrice")]
    effective_gas_price: EffectiveGasPrice,
    status: u32,
    r#type: u32,
    byzantium: bool,
    events: Vec<Log>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Log {
    #[serde(rename = "transactionIndex")]
    transaction_index: u32,
    #[serde(rename = "blockNumber")]
    block_number: u32,
    #[serde(rename = "transactionHash")]
    transaction_hash: String,
    address: String,
    topics: Vec<String>,
    data: String,
    #[serde(rename = "logIndex")]
    log_index: u32,
    #[serde(rename = "blockHash")]
    block_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub receipt: Receipt,
    #[serde(rename = "emitterAddress")]
    pub emitter_address: String,
    pub sequence: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub output: Output,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetForeignAddress {
    pub output: AddressOnEth,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddressOnEth {
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateWrappedOutput {
    receipt: Receipt,
    address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateWrappedResponse {
    output: CreateWrappedOutput,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RedeemOnEthOutput {
    pub receipt: Receipt,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RedeemOnEthResponse {
    pub output: RedeemOnEthOutput,
}

// Function to Decode Hex String to Bytes
pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

// Function to Convert Hex String to Address
pub fn hex_to_address(hex: &str) -> Result<Address, anyhow::Error> {
    if !hex.starts_with("0x") {
        return Err(anyhow::anyhow!("invalid address {}", hex));
    };

    let stripped_address = hex.split_at(2).1;

    let bytes = decode_hex(stripped_address).unwrap();
    let mut array = [0u8; 32];
    array[32 - bytes.len()..].copy_from_slice(&bytes);
    let address: Address = Address(array);
    Ok(address)
}
