use crate::prelude::*;
use flow_lib::config::client::NodeData;
use thiserror::Error as ThisError;

#[derive(Debug)]
pub struct Wallet {
    form: Result<Output, WalletError>,
}

#[derive(Deserialize)]
#[serde(tag = "wallet_type")]
enum FormData {
    #[serde(rename = "HARDCODED")]
    HardCoded { wallet_data: String },
    #[serde(rename = "ADAPTER")]
    Adapter { wallet_data: String },
}

#[derive(ThisError, Debug)]
enum WalletError {
    #[error("failed to decode wallet as base58")]
    InvalidBase58,
    #[error(transparent)]
    Form(serde_json::Error),
}

fn adapter_wallet(pubkey: Pubkey) -> Output {
    let mut buf = [0u8; 64];
    buf[32..].copy_from_slice(&pubkey.to_bytes());
    let keypair = Keypair::from_bytes(&buf).expect("correct size, never fail");
    Output { pubkey, keypair }
}

impl FormData {
    fn into_output(self) -> Result<Output, WalletError> {
        match self {
            FormData::Adapter { wallet_data } => {
                let pubkey = wallet_data
                    .parse::<Pubkey>()
                    .map_err(|_| WalletError::InvalidBase58)?;
                Ok(adapter_wallet(pubkey))
            }
            FormData::HardCoded { wallet_data } => {
                let mut buf = [0u8; 64];
                let size = bs58::decode(wallet_data.trim())
                    .into(&mut buf)
                    .map_err(|_| WalletError::InvalidBase58)?;
                if size != buf.len() {
                    return Err(WalletError::InvalidBase58);
                }
                let keypair = Keypair::from_bytes(&buf).expect("correct size, never fail");

                Ok(Output {
                    pubkey: keypair.pubkey(),
                    keypair,
                })
            }
        }
    }
}

impl Wallet {
    fn new(nd: &NodeData) -> Self {
        let form = serde_json::from_value::<FormData>(nd.targets_form.form_data.clone())
            .map_err(WalletError::Form)
            .and_then(FormData::into_output);
        Self { form }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::pubkey")]
    pub pubkey: Pubkey,
    #[serde(with = "value::keypair")]
    pub keypair: Keypair,
}

const WALLET: &str = "wallet";

#[async_trait]
impl CommandTrait for Wallet {
    fn name(&self) -> Name {
        WALLET.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [].to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [
            CmdOutput {
                name: "pubkey".into(),
                r#type: ValueType::Pubkey,
            },
            CmdOutput {
                name: "keypair".into(),
                r#type: ValueType::Keypair,
            },
        ]
        .to_vec()
    }

    async fn run(&self, _: Context, _: ValueSet) -> Result<ValueSet, CommandError> {
        match &self.form {
            Ok(output) => Ok(value::to_map(output)?),
            Err(e) => Err(CommandError::msg(e.to_string())),
        }
    }
}

inventory::submit!(CommandDescription::new(WALLET, |nd| {
    Ok(Box::new(Wallet::new(nd)))
}));

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::config::client::{Extra, TargetsForm};
    use serde_json::json;

    const PUBKEY: Pubkey = solana_sdk::pubkey!("DKsvmM9hfNm4R94yB3VdYMZJk2ETv5hpcjuRmiwgiztY");
    const PUBKEY_STR: &str = "DKsvmM9hfNm4R94yB3VdYMZJk2ETv5hpcjuRmiwgiztY";

    #[test]
    fn adapter() {
        let nd = NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: WALLET.into(),
            sources: Vec::new(),
            targets: Vec::new(),
            targets_form: TargetsForm {
                form_data: json!({
                    // there is also "wallet_id", but it is not used
                    "wallet_type": "ADAPTER",
                    "wallet_data": PUBKEY_STR,
                }),
                extra: Extra::default(),
                wasm_bytes: None,
            },
        };
        assert_eq!(Wallet::new(&nd).form.unwrap().pubkey, PUBKEY);
    }

    #[test]
    fn hardcoded() {
        const KEYPAIR: &str = "oLXLpXdGn6RjMHz3fvcPdGNUDQxXu91t7YAFbtRew3TFVPHAU1UrZJpgiHDLKDtrWZRQg6trQFFp6zEX2TQ1S3k";

        let nd = NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: WALLET.into(),
            sources: Vec::new(),
            targets: Vec::new(),
            targets_form: TargetsForm {
                form_data: json!({
                    // there is also "wallet_id", but it is not used
                    "wallet_type": "HARDCODED",
                    "wallet_data": KEYPAIR,
                }),
                extra: Extra::default(),
                wasm_bytes: None,
            },
        };
        let wallet = Wallet::new(&nd).form.unwrap();
        assert_eq!(wallet.keypair.to_base58_string(), KEYPAIR);
        assert_eq!(wallet.keypair.pubkey(), wallet.pubkey);
    }
}
