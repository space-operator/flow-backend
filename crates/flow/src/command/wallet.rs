use crate::command::prelude::*;
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use chacha20poly1305::{
    AeadCore, ChaCha20Poly1305, KeyInit,
    aead::{Aead, rand_core},
};
use flow_lib::{
    CmdInputDescription, CmdOutputDescription,
    config::{WalletId, client::NodeData},
    solana::{Pubkey, Wallet},
};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use zeroize::ZeroizeOnDrop;

#[derive(Debug)]
struct WalletCmd {
    form: Result<FormData, serde_json::Error>,
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
        let form = parse_wallet_form(nd.targets_form.form_data.clone());
        Self { form }
    }
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

const WALLET: &str = "wallet";

#[async_trait(?Send)]
impl CommandTrait for WalletCmd {
    fn name(&self) -> Name {
        WALLET.into()
    }

    fn inputs(&self) -> Vec<CmdInputDescription> {
        [].to_vec()
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

    async fn run(&self, _: CommandContext, _: ValueSet) -> Result<ValueSet, CommandError> {
        match &self.form {
            Ok(form) => {
                /*
                let permit = ctx
                    .get::<WalletPermit>()
                    .ok_or_else(|| CommandError::msg("WalletPermit not found"))?;
                let token = permit.encrypt(form.wallet_id);
                */
                let output = Output {
                    pubkey: form.public_key,
                    keypair: Wallet::Adapter {
                        public_key: form.public_key,
                        token: None,
                    },
                };

                Ok(value::to_map(&output)?)
            }
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

    const PUBKEY_STR: &str = "DKsvmM9hfNm4R94yB3VdYMZJk2ETv5hpcjuRmiwgiztY";

    #[test]
    fn rejects_plain_json_form() {
        let nd = NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: WALLET.into(),
            sources: Vec::new(),
            targets: Vec::new(),
            targets_form: TargetsForm {
                form_data: json!({
                    "public_key": PUBKEY_STR,
                    "wallet_id": 0,
                }),
                extra: Extra::default(),
                wasm_bytes: None,
            },
            instruction_info: None,
        };
        assert!(WalletCmd::new(&nd).form.is_err());
    }

    #[test]
    fn adapter_accepts_ivalue_config() {
        let nd = NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: WALLET.into(),
            sources: Vec::new(),
            targets: Vec::new(),
            targets_form: TargetsForm {
                form_data: json!({
                    "public_key": { "B3": PUBKEY_STR },
                    "wallet_id": { "U": "7" }
                }),
                extra: Extra::default(),
                wasm_bytes: None,
            },
            instruction_info: None,
        };

        assert!(matches!(
            WalletCmd::new(&nd).form,
            Ok(FormData {
                public_key,
                wallet_id: 7
            }) if public_key == Pubkey::from_str_const(PUBKEY_STR)
        ));
    }
}
