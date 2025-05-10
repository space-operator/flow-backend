use crate::prelude::*;
use flow_lib::config::client::NodeData;
use thiserror::Error as ThisError;

#[derive(Debug)]
pub struct WalletCmd {
    form: Result<Output, WalletError>,
}

#[derive(Deserialize)]
struct FormData {
    public_key: String,
}

#[derive(ThisError, Debug)]
enum WalletError {
    #[error("failed to decode wallet as base58")]
    InvalidBase58,
    #[error(transparent)]
    Form(serde_json::Error),
}

fn adapter_wallet(pubkey: Pubkey) -> Output {
    Output {
        pubkey,
        keypair: Wallet::Adapter { public_key: pubkey },
    }
}

impl FormData {
    fn into_output(self) -> Result<Output, WalletError> {
        let pubkey = self
            .public_key
            .parse::<Pubkey>()
            .map_err(|_| WalletError::InvalidBase58)?;
        Ok(adapter_wallet(pubkey))
    }
}

impl WalletCmd {
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
    pub keypair: Wallet,
}

const WALLET: &str = "wallet";

#[async_trait]
impl CommandTrait for WalletCmd {
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
                optional: false,
            },
            CmdOutput {
                name: "keypair".into(),
                r#type: ValueType::Keypair,
                optional: false,
            },
        ]
        .to_vec()
    }

    async fn run(&self, _: CommandContext, _: ValueSet) -> Result<ValueSet, CommandError> {
        match &self.form {
            Ok(output) => Ok(value::to_map(output)?),
            Err(e) => Err(CommandError::msg(e.to_string())),
        }
    }
}

flow_lib::submit!(CommandDescription::new(WALLET, |nd| {
    Ok(Box::new(WalletCmd::new(nd)))
}));

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::config::client::{Extra, TargetsForm};
    use serde_json::json;

    const PUBKEY: Pubkey = solana_program::pubkey!("DKsvmM9hfNm4R94yB3VdYMZJk2ETv5hpcjuRmiwgiztY");
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
                    "public_key": PUBKEY_STR,
                }),
                extra: Extra::default(),
                wasm_bytes: None,
            },
            instruction_info: None,
        };
        assert_eq!(WalletCmd::new(&nd).form.unwrap().pubkey, PUBKEY);
    }
}
