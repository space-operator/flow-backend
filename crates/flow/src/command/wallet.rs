use crate::command::prelude::*;
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use chacha20poly1305::{
    AeadCore, ChaCha20Poly1305, KeyInit,
    aead::{Aead, rand_core},
};
use flow_lib::{
    CmdInputDescription, CmdOutputDescription,
    config::{WalletId, client::NodeData},
    solana::{Pubkey, Wallet, WalletOrPubkey},
};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use zeroize::ZeroizeOnDrop;

#[derive(Debug)]
struct WalletCmd {
    // Static config remains a fallback even when runtime or edge input is enabled.
    form: Result<FormData, serde_json::Error>,
    // `api_input` only gates start/deployment input binding; edge input always stays available.
    api_input: bool,
    // Support the old label-based start input during migration, but map it into `wallet`.
    legacy_api_label: Option<Name>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub(crate) struct FormData {
    #[serde_as(as = "DisplayFromStr")]
    public_key: Pubkey,
    wallet_id: i64,
}

impl WalletCmd {
    fn new(nd: &NodeData) -> Self {
        let api_input = nd
            .config
            .get("api_input")
            .and_then(flow_lib::command::parse_value_tagged_bool)
            .unwrap_or(false);

        let legacy_api_label = nd.config.get("label").and_then(parse_string_value).filter(|label| {
            !label.is_empty() && label != INPUT_WALLET
        });

        Self {
            form: parse_wallet_form(nd.config.clone()),
            api_input,
            legacy_api_label,
        }
    }
}

fn wallet_from_input(value: Value) -> Result<Wallet, CommandError> {
    Ok(value::from_value::<WalletOrPubkey>(value)?.to_keypair())
}

fn invalid_data_error(message: &str) -> serde_json::Error {
    serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.to_owned(),
    ))
}

fn parse_pubkey(json: &JsonValue) -> Option<Pubkey> {
    flow_lib::command::parse_value_tagged(json.clone())
        .ok()
        .and_then(|value| value::pubkey::deserialize(value).ok())
}

fn parse_string_value(json: &JsonValue) -> Option<String> {
    flow_lib::command::parse_value_tagged(json.clone())
        .ok()
        .and_then(|value| match value {
            Value::String(s) if !s.is_empty() => Some(s),
            _ => None,
        })
}

fn parse_wallet_id(json: &JsonValue) -> Option<i64> {
    flow_lib::command::parse_value_tagged(json.clone())
        .ok()
        .and_then(|value| match value {
            Value::I64(v) => Some(v),
            Value::U64(v) => i64::try_from(v).ok(),
            _ => None,
        })
}

fn parse_wallet_form(json: JsonValue) -> Result<FormData, serde_json::Error> {
    let public_key = json
        .get("public_key")
        .and_then(parse_pubkey)
        .ok_or_else(|| invalid_data_error("wallet.public_key"))?;
    let wallet_id = json
        .get("wallet_id")
        .and_then(parse_wallet_id)
        .ok_or_else(|| invalid_data_error("wallet.wallet_id"))?;

    Ok(FormData {
        public_key,
        wallet_id,
    })
}

#[derive(ZeroizeOnDrop)]
pub(crate) struct WalletPermit {
    encryption_key: ChaCha20Poly1305,
}

#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
struct Permit {
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}

impl WalletPermit {
    pub(crate) fn new() -> Self {
        Self {
            encryption_key: ChaCha20Poly1305::new(&ChaCha20Poly1305::generate_key(
                &mut rand_core::OsRng,
            )),
        }
    }

    fn encrypt(&self, wallet_id: WalletId) -> Result<String, CommandError> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut rand_core::OsRng);
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(nonce.as_slice());
        let borsh_form = borsh::to_vec(&wallet_id)?;
        let ciphertext = self.encryption_key.encrypt(&nonce, borsh_form.as_slice())?;
        let permit = Permit {
            nonce: nonce_bytes,
            ciphertext,
        };
        let encoded = borsh::to_vec(&permit)?;
        Ok(BASE64_STANDARD_NO_PAD.encode(encoded))
    }

    pub(crate) fn decrypt(&self, base64_permit: &str) -> Result<WalletId, CommandError> {
        let borsh_permit = BASE64_STANDARD_NO_PAD.decode(base64_permit)?;
        let permit: Permit = borsh::from_slice(&borsh_permit)?;
        let borsh_form = self
            .encryption_key
            .decrypt(&permit.nonce.into(), permit.ciphertext.as_slice())?;
        let wallet_id: WalletId = borsh::from_slice(&borsh_form)?;
        Ok(wallet_id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    #[serde(with = "value::pubkey")]
    pub pubkey: Pubkey,
    pub keypair: Wallet,
}

pub const WALLET: &str = "wallet";
const INPUT_WALLET: &str = "wallet";

fn wallet_from_form(form: &FormData) -> Wallet {
    Wallet::Adapter {
        public_key: form.public_key,
        token: None,
    }
}

#[async_trait(?Send)]
impl CommandTrait for WalletCmd {
    fn name(&self) -> Name {
        WALLET.into()
    }

    fn inputs(&self) -> Vec<CmdInputDescription> {
        vec![CmdInputDescription {
            // Keep the wallet input stable so graph edges and start inputs share one contract.
            name: INPUT_WALLET.into(),
            type_bounds: vec![ValueType::Keypair, ValueType::Pubkey],
            required: false,
            passthrough: false,
        }]
    }

    fn outputs(&self) -> Vec<CmdOutputDescription> {
        [
            CmdOutputDescription {
                name: "pubkey".into(),
                r#type: ValueType::Pubkey,
                optional: false,
            },
            CmdOutputDescription {
                name: "keypair".into(),
                r#type: ValueType::Keypair,
                optional: false,
            },
        ]
        .to_vec()
    }

    async fn run(&self, _: CommandContext, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let wallet = match inputs.swap_remove(INPUT_WALLET) {
            // Null keeps the fallback chain intact for optional edge/start bindings.
            Some(Value::Null) | None => match &self.form {
                Ok(form) => wallet_from_form(form),
                Err(error) => {
                    return Err(CommandError::msg(format!(
                        "wallet requires either an input wallet or valid static wallet config: {error}"
                    )));
                }
            },
            Some(value) => wallet_from_input(value)?,
        };

        let output = Output {
            pubkey: wallet.pubkey(),
            keypair: wallet,
        };
        Ok(value::to_map(&output)?)
    }

    fn read_config(&self, data: JsonValue) -> ValueSet {
        if let Some(value) = data.get(INPUT_WALLET) {
            return value::map! {
                INPUT_WALLET => flow_lib::command::parse_value_tagged_or_json(value.clone()),
            };
        }

        if let Some(value) = data.get("value") {
            // Keep the legacy config alias readable while callers migrate to `wallet`.
            return value::map! {
                INPUT_WALLET => flow_lib::command::parse_value_tagged_or_json(value.clone()),
            };
        }

        ValueSet::new()
    }

    fn bind_flow_inputs(&self, flow_inputs: &ValueSet) -> ValueSet {
        if !self.api_input {
            return ValueSet::new();
        }

        let value = flow_inputs
            .get(INPUT_WALLET)
            .cloned()
            .or_else(|| {
                self.legacy_api_label
                    .as_ref()
                    .and_then(|label| flow_inputs.get(label).cloned())
            });

        match value {
            Some(value) => value::map! {
                INPUT_WALLET => value,
            },
            None => ValueSet::new(),
        }
    }
}

flow_lib::submit!(CommandDescription::new(WALLET, |nd| {
    Ok(Box::new(WalletCmd::new(nd)))
}));

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::config::client::OutputPort;
    use serde_json::json;
    use uuid::Uuid;

    const PUBKEY_STR: &str = "DKsvmM9hfNm4R94yB3VdYMZJk2ETv5hpcjuRmiwgiztY";

    fn test_node(config: JsonValue) -> NodeData {
        NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: WALLET.into(),
            outputs: vec![OutputPort {
                id: Uuid::new_v4(),
                name: "pubkey".to_owned(),
                r#type: ValueType::Pubkey,
                optional: false,
                tooltip: None,
            }],
            inputs: Vec::new(),
            config,
            wasm: None,
            instruction_info: None,
        }
    }

    #[test]
    fn rejects_plain_json_form() {
        let nd = test_node(json!({
            "public_key": PUBKEY_STR,
            "wallet_id": 0,
        }));
        assert!(WalletCmd::new(&nd).form.is_err());
    }

    #[test]
    fn adapter_accepts_ivalue_config() {
        let nd = test_node(json!({
            "public_key": { "B3": PUBKEY_STR },
            "wallet_id": { "U": "7" }
        }));

        assert!(matches!(
            &WalletCmd::new(&nd).form,
            Ok(FormData {
                public_key,
                wallet_id: 7
            }) if *public_key == Pubkey::from_str_const(PUBKEY_STR)
        ));
    }

    #[test]
    fn stable_wallet_input_is_always_declared() {
        let nd = test_node(json!({
            "api_input": { "B": true },
        }));
        let cmd = WalletCmd::new(&nd);
        assert_eq!(cmd.inputs().len(), 1);
        assert_eq!(cmd.inputs()[0].name, INPUT_WALLET);
        assert_eq!(
            cmd.inputs()[0].type_bounds,
            vec![ValueType::Keypair, ValueType::Pubkey]
        );
    }

    #[tokio::test]
    async fn run_uses_static_wallet_when_no_input() {
        let nd = test_node(json!({
            "public_key": { "B3": PUBKEY_STR },
            "wallet_id": { "U": "7" }
        }));
        let cmd = WalletCmd::new(&nd);
        let output = cmd
            .run(CommandContext::default(), ValueSet::new())
            .await
            .unwrap();
        let output: Output = value::from_map(output).unwrap();
        assert_eq!(output.pubkey, Pubkey::from_str_const(PUBKEY_STR));
        assert!(output.keypair.is_adapter_wallet());
    }

    #[tokio::test]
    async fn keypair_input_overrides_static_wallet() {
        let nd = test_node(json!({
            "public_key": { "B3": PUBKEY_STR },
            "wallet_id": { "U": "7" }
        }));
        let cmd = WalletCmd::new(&nd);
        let wallet = Wallet::Keypair(flow_lib::solana::Keypair::new());
        let pubkey = wallet.pubkey();

        let inputs = value::map! {
            INPUT_WALLET => value::to_value(&wallet).unwrap(),
        };

        let output = cmd.run(CommandContext::default(), inputs).await.unwrap();
        let output: Output = value::from_map(output).unwrap();
        assert_eq!(output.pubkey, pubkey);
        assert_eq!(output.keypair.pubkey(), pubkey);
    }

    #[tokio::test]
    async fn pubkey_input_overrides_static_wallet() {
        let nd = test_node(json!({
            "public_key": { "B3": PUBKEY_STR },
            "wallet_id": { "U": "7" }
        }));
        let cmd = WalletCmd::new(&nd);
        let pubkey = Pubkey::new_unique();

        let output = cmd
            .run(
                CommandContext::default(),
                value::map! {
                    INPUT_WALLET => value::to_value(&pubkey).unwrap(),
                },
            )
            .await
            .unwrap();
        let output: Output = value::from_map(output).unwrap();
        assert_eq!(output.pubkey, pubkey);
        assert!(output.keypair.is_adapter_wallet());
    }

    #[tokio::test]
    async fn null_input_falls_back_to_static_wallet() {
        let nd = test_node(json!({
            "public_key": { "B3": PUBKEY_STR },
            "wallet_id": { "U": "7" }
        }));
        let cmd = WalletCmd::new(&nd);
        let output = cmd
            .run(
                CommandContext::default(),
                value::map! {
                    INPUT_WALLET => Value::Null,
                },
            )
            .await
            .unwrap();
        let output: Output = value::from_map(output).unwrap();
        assert_eq!(output.pubkey, Pubkey::from_str_const(PUBKEY_STR));
        assert!(output.keypair.is_adapter_wallet());
    }

    #[tokio::test]
    async fn missing_all_sources_errors_clearly() {
        let cmd = WalletCmd::new(&test_node(json!({
            "api_input": { "B": true },
        })));
        let error = cmd
            .run(CommandContext::default(), ValueSet::new())
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("wallet requires either an input wallet or valid static wallet config"));
    }

    #[test]
    fn bind_flow_inputs_reads_wallet_when_api_input_is_enabled() {
        let cmd = WalletCmd::new(&test_node(json!({
            "api_input": { "B": true },
        })));
        let pubkey = Pubkey::new_unique();
        let values = cmd.bind_flow_inputs(&value::map! {
            INPUT_WALLET => value::to_value(&pubkey).unwrap(),
        });
        assert_eq!(values.get(INPUT_WALLET), Some(&value::to_value(&pubkey).unwrap()));
    }

    #[test]
    fn bind_flow_inputs_ignores_wallet_when_api_input_is_disabled() {
        let cmd = WalletCmd::new(&test_node(json!({})));
        let values = cmd.bind_flow_inputs(&value::map! {
            INPUT_WALLET => Value::Null,
        });
        assert!(values.is_empty());
    }

    #[test]
    fn bind_flow_inputs_supports_legacy_label_alias() {
        let cmd = WalletCmd::new(&test_node(json!({
            "api_input": { "B": true },
            "label": { "S": "fee_payer" },
        })));
        let pubkey = Pubkey::new_unique();
        let values = cmd.bind_flow_inputs(&value::map! {
            "fee_payer" => value::to_value(&pubkey).unwrap(),
        });
        assert_eq!(values.get(INPUT_WALLET), Some(&value::to_value(&pubkey).unwrap()));
    }

    #[test]
    fn read_config_supports_legacy_value_alias() {
        let cmd = WalletCmd::new(&test_node(json!({})));
        let values = cmd.read_config(json!({
            "value": { "B3": PUBKEY_STR },
        }));
        assert_eq!(
            values.get(INPUT_WALLET),
            Some(&Value::String(PUBKEY_STR.to_owned()))
        );
    }

    #[test]
    fn read_config_prefers_canonical_wallet_field() {
        let cmd = WalletCmd::new(&test_node(json!({})));
        let values = cmd.read_config(json!({
            "wallet": { "B3": PUBKEY_STR },
            "value": { "B3": Pubkey::new_unique().to_string() },
        }));
        assert_eq!(
            values.get(INPUT_WALLET),
            Some(&Value::String(PUBKEY_STR.to_owned()))
        );
    }
}
