use crate::command::prelude::*;
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use chacha20poly1305::{
    AeadCore, ChaCha20Poly1305, KeyInit,
    aead::{Aead, rand_core},
};
use flow_lib::{
    CmdInputDescription, CmdOutputDescription,
    config::client::NodeData,
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
        let form = serde_json::from_value::<FormData>(nd.targets_form.form_data.clone());
        Self { form }
    }
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

    fn encrypt(&self, wallet: &FormData) -> String {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut rand_core::OsRng);
        let borsh_form = borsh::to_vec(wallet).unwrap();
        let ciphertext = self
            .encryption_key
            .encrypt(&nonce, borsh_form.as_slice())
            .unwrap();
        let permit = Permit {
            nonce: *nonce.as_array().unwrap(),
            ciphertext,
        };
        BASE64_STANDARD_NO_PAD.encode(&borsh::to_vec(&permit).unwrap())
    }

    pub(crate) fn decrypt(&self, base64_permit: &str) -> Result<FormData, CommandError> {
        let borsh_permit = BASE64_STANDARD_NO_PAD.decode(base64_permit)?;
        let permit: Permit = borsh::from_slice(&borsh_permit)?;
        let borsh_form = self
            .encryption_key
            .decrypt(&permit.nonce.into(), permit.ciphertext.as_slice())?;
        let form: FormData = borsh::from_slice(&borsh_form)?;
        Ok(form)
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

    async fn run(&self, ctx: CommandContext, _: ValueSet) -> Result<ValueSet, CommandError> {
        match &self.form {
            Ok(form) => {
                let permit = ctx
                    .get::<WalletPermit>()
                    .ok_or_else(|| CommandError::msg("WalletPermit not found"))?;
                let token = permit.encrypt(form);
                let output = Output {
                    pubkey: form.public_key,
                    keypair: Wallet::Adapter {
                        public_key: form.public_key,
                        token: Some(token),
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
        assert_eq!(
            WalletCmd::new(&nd).form.unwrap().public_key,
            Pubkey::from_str_const(PUBKEY_STR)
        );
    }
}
