use crate::SolanaNet;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as, serde_conv};
use solana_commitment_config::CommitmentLevel;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_signer::Signer;
use std::{
    borrow::Cow, collections::HashMap, convert::Infallible, fmt::Display, num::ParseIntError,
    str::FromStr, time::Duration,
};
use value::{
    Value,
    with::{AsKeypair, AsPubkey},
};

pub use solana_keypair::Keypair;
pub use solana_pubkey::Pubkey;
pub use solana_signature::Signature;

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(3 * 60);

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Wallet {
    Keypair(#[serde_as(as = "AsKeypair")] Keypair),
    Adapter {
        #[serde_as(as = "AsPubkey")]
        public_key: Pubkey,
    },
}

impl bincode::Encode for Wallet {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        WalletBincode::from(self).encode(encoder)
    }
}

impl<C> bincode::Decode<C> for Wallet {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(WalletBincode::decode(decoder)?.into())
    }
}

impl<'de, C> bincode::BorrowDecode<'de, C> for Wallet {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(WalletBincode::borrow_decode(decoder)?.into())
    }
}

#[derive(bincode::Encode, bincode::Decode)]
enum WalletBincode {
    Keypair([u8; 32]),
    Adapter([u8; 32]),
}

impl From<WalletBincode> for Wallet {
    fn from(value: WalletBincode) -> Self {
        match value {
            WalletBincode::Keypair(value) => Wallet::Keypair(Keypair::new_from_array(value)),
            WalletBincode::Adapter(value) => Wallet::Adapter {
                public_key: Pubkey::new_from_array(value),
            },
        }
    }
}

impl From<&Wallet> for WalletBincode {
    fn from(value: &Wallet) -> Self {
        match value {
            Wallet::Keypair(keypair) => WalletBincode::Keypair(*keypair.secret_bytes()),
            Wallet::Adapter { public_key } => WalletBincode::Adapter(public_key.to_bytes()),
        }
    }
}

impl From<Keypair> for Wallet {
    fn from(value: Keypair) -> Self {
        Self::Keypair(value)
    }
}

impl Clone for Wallet {
    fn clone(&self) -> Self {
        match self {
            Wallet::Keypair(keypair) => Wallet::Keypair(keypair.insecure_clone()),
            Wallet::Adapter { public_key } => Wallet::Adapter {
                public_key: *public_key,
            },
        }
    }
}

impl Wallet {
    pub fn is_adapter_wallet(&self) -> bool {
        matches!(self, Wallet::Adapter { .. })
    }

    pub fn pubkey(&self) -> Pubkey {
        match self {
            Wallet::Keypair(keypair) => keypair.pubkey(),
            Wallet::Adapter { public_key, .. } => *public_key,
        }
    }

    pub fn keypair(&self) -> Option<&Keypair> {
        match self {
            Wallet::Keypair(keypair) => Some(keypair),
            Wallet::Adapter { .. } => None,
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
struct AsAccountMetaImpl {
    #[serde_as(as = "AsPubkey")]
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}
fn account_meta_ser(i: &AccountMeta) -> AsAccountMetaImpl {
    AsAccountMetaImpl {
        pubkey: i.pubkey,
        is_signer: i.is_signer,
        is_writable: i.is_writable,
    }
}
fn account_meta_de(i: AsAccountMetaImpl) -> Result<AccountMeta, Infallible> {
    Ok(AccountMeta {
        pubkey: i.pubkey,
        is_signer: i.is_signer,
        is_writable: i.is_writable,
    })
}
serde_conv!(
    AsAccountMeta,
    AccountMeta,
    account_meta_ser,
    account_meta_de
);

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
struct AsInstructionImpl {
    #[serde_as(as = "AsPubkey")]
    program_id: Pubkey,
    #[serde_as(as = "Vec<AsAccountMeta>")]
    accounts: Vec<AccountMeta>,
    #[serde_as(as = "serde_with::Bytes")]
    data: Vec<u8>,
}
fn instruction_ser(i: &Instruction) -> AsInstructionImpl {
    AsInstructionImpl {
        program_id: i.program_id,
        accounts: i.accounts.clone(),
        data: i.data.clone(),
    }
}
fn instruction_de(i: AsInstructionImpl) -> Result<Instruction, Infallible> {
    Ok(Instruction {
        program_id: i.program_id,
        accounts: i.accounts,
        data: i.data,
    })
}
serde_conv!(AsInstruction, Instruction, instruction_ser, instruction_de);

#[serde_as]
#[derive(
    Serialize, Deserialize, Debug, Default, bon::Builder, bincode::Encode, bincode::Decode,
)]
pub struct Instructions {
    #[serde_as(as = "AsPubkey")]
    #[bincode(with_serde)]
    pub fee_payer: Pubkey,
    pub signers: Vec<Wallet>,
    #[serde_as(as = "Vec<AsInstruction>")]
    #[bincode(with_serde)]
    pub instructions: Vec<Instruction>,
    #[serde_as(as = "Option<Vec<AsPubkey>>")]
    #[bincode(with_serde)]
    pub lookup_tables: Option<Vec<Pubkey>>,
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum InsertionBehavior {
    #[default]
    Auto,
    No,
    Value(u64),
}

impl FromStr for InsertionBehavior {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "auto" => InsertionBehavior::Auto,
            "no" => InsertionBehavior::No,
            s => InsertionBehavior::Value(s.parse()?),
        })
    }
}

impl Display for InsertionBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertionBehavior::Auto => f.write_str("auto"),
            InsertionBehavior::No => f.write_str("no"),
            InsertionBehavior::Value(v) => v.fmt(f),
        }
    }
}

impl Serialize for InsertionBehavior {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InsertionBehavior {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        <Cow<'de, str> as Deserialize>::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

const fn default_simulation_level() -> CommitmentLevel {
    CommitmentLevel::Finalized
}

const fn default_tx_level() -> CommitmentLevel {
    CommitmentLevel::Confirmed
}

const fn default_wait_level() -> CommitmentLevel {
    CommitmentLevel::Confirmed
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum WalletOrPubkey {
    Wallet(Wallet),
    Pubkey(#[serde_as(as = "AsPubkey")] Pubkey),
}

impl WalletOrPubkey {
    pub fn to_keypair(self) -> Wallet {
        match self {
            WalletOrPubkey::Wallet(k) => k,
            WalletOrPubkey::Pubkey(public_key) => Wallet::Adapter { public_key },
        }
    }
}

#[serde_with::serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ExecutionConfig {
    pub overwrite_feepayer: Option<WalletOrPubkey>,

    pub devnet_lookup_table: Option<Pubkey>,
    pub mainnet_lookup_table: Option<Pubkey>,

    #[serde(default)]
    pub compute_budget: InsertionBehavior,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fallback_compute_budget: Option<u64>,
    #[serde(default)]
    pub priority_fee: InsertionBehavior,

    #[serde(default = "default_simulation_level")]
    pub simulation_commitment_level: CommitmentLevel,
    #[serde(default = "default_tx_level")]
    pub tx_commitment_level: CommitmentLevel,
    #[serde(default = "default_wait_level")]
    pub wait_commitment_level: CommitmentLevel,

    #[serde(skip)]
    pub execute_on: ExecuteOn,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SolanaActionConfig {
    #[serde(with = "value::pubkey")]
    pub action_signer: Pubkey,
    #[serde(with = "value::pubkey")]
    pub action_identity: Pubkey,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub enum ExecuteOn {
    SolanaAction(SolanaActionConfig),
    #[default]
    CurrentMachine,
}

impl ExecutionConfig {
    pub fn from_env(map: &HashMap<String, String>) -> Result<Self, value::Error> {
        let map = map
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect::<value::Map>();
        value::from_map(map)
    }

    pub fn lookup_table(&self, network: SolanaNet) -> Option<Pubkey> {
        match network {
            SolanaNet::Devnet => self.devnet_lookup_table,
            SolanaNet::Testnet => None,
            SolanaNet::Mainnet => self.mainnet_lookup_table,
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            overwrite_feepayer: None,
            devnet_lookup_table: None,
            mainnet_lookup_table: None,
            compute_budget: InsertionBehavior::default(),
            fallback_compute_budget: None,
            priority_fee: InsertionBehavior::default(),
            simulation_commitment_level: default_simulation_level(),
            tx_commitment_level: default_tx_level(),
            wait_commitment_level: default_wait_level(),
            execute_on: ExecuteOn::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::env::{
        COMPUTE_BUDGET, FALLBACK_COMPUTE_BUDGET, OVERWRITE_FEEPAYER, PRIORITY_FEE,
        SIMULATION_COMMITMENT_LEVEL, TX_COMMITMENT_LEVEL, WAIT_COMMITMENT_LEVEL,
    };
    use bincode::config::standard;
    // use base64::prelude::*;
    use solana_program::{pubkey, system_instruction::transfer};

    #[test]
    fn test_wallet_serde() {
        let keypair = Keypair::new();
        let input = Value::String(keypair.to_base58_string());
        let Wallet::Keypair(result) = value::from_value(input).unwrap() else {
            panic!()
        };
        assert_eq!(result.to_base58_string(), keypair.to_base58_string());
    }

    /* TODO: add this test back
     * failed because it is a "legacy" tx, we are using "v0" tx
    #[test]
    fn test_compare_msg_logic() {
        const OLD: &str = "AwEJE/I9QMIByO+GhMkfll9MXSsAYs1ITPmKAfxGS/USlNwuw0EUt8a41tLSp95YmtHPKWDGGcApBC0AEmN1Sd+5kfDOAq0G+/qWg2KKmXfDQF1HIuw9Op9LiSZK5iA7jcVQ9wceNyYLLzZIZ+cVomhs1zT04hQeIKdXkiMyUpH9KA95JukMx1A93RFsivUbXmW+wwO52yE0+21NxUpXL/eMTCpS1wQ6IUwmvO0o13hn6qE0Pi73WxtEGjlbBilP+HVyqFkAIKLtjJBJ25Jae9iO3Xe17TFanfbTgtEbgKAJ5nWVuJt84ctKVWEXbuPgqHbe6H8fchmNtE0iKLjuVOE0AJ3GIRyraKaGg0wqZXXkbS0qr6CQYxZVv7PeO7zsL/swgPucBbMHhqVF+Mv8NimuycfvB72jxeN3uhwn+c715MdKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADBkZv5SEXMv/srbpyw5vnvIzlu8X3EmssQ5s6QAAAAAan1RcYe9FmNdrUBFX9wsDBJMaPIVZ1pdu6y18IAAAABt324ddloZPZy+FGzut5rBy0he1fWzeROoz1hX7/AKkLcGWx49F8RTidUn9rBMPNWLhscxqg/bVJttG8A/gpRlM2SFRbPsgTT3LuOBLPsJzpVN5CeDaecGGyxbawEE6Kcy72NeMo2v4ccHESWqcHq3GioOBRqLHY25fQEpaeCVSLCKI3/q1QflOctOQHXPk3VuQhThJQPfn/dD3sEZbonYyXJY9OJInxuz0QKRSODYMLWhOZ2v8QhASOe9jb6fhZdtEfrjiMo8c/EYJzRiXnOLehdv4i42eBpdbr4NYTAzkICwAJA+gDAAAAAAAACwAFAkANAwAOCQMFAQIAAgoMDdoBKgAYAAAAU3BhY2UgT3BlcmF0b3IgQ2hhbWVsZW9uBAAAAFNQT0NTAAAAaHR0cHM6Ly9hc3NldHMuc3BhY2VvcGVyYXRvci5jb20vbWV0YWRhdGEvMzU4NjY4MzItN2M4My00OWM2LWJmZjctY2FhMDBiNmE2NDE1Lmpzb276AAEBAAAAzgKtBvv6loNiipl3w0BdRyLsPTqfS4kmSuYgO43FUPcAZAABBAEAiwiiN/6tUH5TnLTkB1z5N1bkIU4SUD35/3Q97BGW6J0AAAABAAEBZAAAAAAAAAAOCAIOAxEJDwoMAjQBDggCDgMODg4KDAI0AA4OBxADBQQBCAIACgwNDg4DLAMADg8IAAMFBAECDgAKDA0SDg4LKwABAAAAAAAAAAAKAgAGDAIAAAAAu+6gAAAAAA==";
        const NEW: &str = "AwEJE/I9QMIByO+GhMkfll9MXSsAYs1ITPmKAfxGS/USlNwuw0EUt8a41tLSp95YmtHPKWDGGcApBC0AEmN1Sd+5kfDOAq0G+/qWg2KKmXfDQF1HIuw9Op9LiSZK5iA7jcVQ9ybpDMdQPd0RbIr1G15lvsMDudshNPttTcVKVy/3jEwqUtcEOiFMJrztKNd4Z+qhND4u91sbRBo5WwYpT/h1cqhZACCi7YyQSduSWnvYjt13te0xWp3204LRG4CgCeZ1lbibfOHLSlVhF27j4Kh23uh/H3IZjbRNIii47lThNACdxiEcq2imhoNMKmV15G0tKq+gkGMWVb+z3ju87C/7MID7nAWzB4alRfjL/DYprsnH7we9o8Xjd7ocJ/nO9eTHSgceNyYLLzZIZ+cVomhs1zT04hQeIKdXkiMyUpH9KA95AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABTNkhUWz7IE09y7jgSz7Cc6VTeQng2nnBhssW2sBBOinMu9jXjKNr+HHBxElqnB6txoqDgUaix2NuX0BKWnglUiwiiN/6tUH5TnLTkB1z5N1bkIU4SUD35/3Q97BGW6J2MlyWPTiSJ8bs9ECkUjg2DC1oTmdr/EIQEjnvY2+n4WQMGRm/lIRcy/+ytunLDm+e8jOW7xfcSayxDmzpAAAAAC3BlsePRfEU4nVJ/awTDzVi4bHMaoP21SbbRvAP4KUYGp9UXGHvRZjXa1ARV/cLAwSTGjyFWdaXbustfCAAAAAbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpdtEfrjiMo8c/EYJzRiXnOLehdv4i42eBpdbr4NYTAzkIDwAJA+gDAAAAAAAADwAFAkANAwAQCQkEAQIAAgoREtoBKgAYAAAAU3BhY2UgT3BlcmF0b3IgQ2hhbWVsZW9uBAAAAFNQT0NTAAAAaHR0cHM6Ly9hc3NldHMuc3BhY2VvcGVyYXRvci5jb20vbWV0YWRhdGEvMzU4NjY4MzItN2M4My00OWM2LWJmZjctY2FhMDBiNmE2NDE1Lmpzb276AAEBAAAAzgKtBvv6loNiipl3w0BdRyLsPTqfS4kmSuYgO43FUPcAZAABBAEAiwiiN/6tUH5TnLTkB1z5N1bkIU4SUD35/3Q97BGW6J0AAAABAAEBZAAAAAAAAAAQCAIQCQ0ICwoRAjQBEAgCEAkQEBAKEQI0ABAOBgwJBAMBBwIAChESEBADLAMAEA8HAAkEAwECEAAKERIOEBALKwABAAAAAAAAAAAKAgAFDAIAAAAAu+6gAAAAAA==";
        is_same_message_logic(
            &BASE64_STANDARD.decode(OLD).unwrap(),
            &BASE64_STANDARD.decode(NEW).unwrap(),
        )
        .unwrap();
    }
    */

    #[test]
    fn test_parse_config() {
        fn t<const N: usize>(kv: [(&str, &str); N], result: ExecutionConfig) {
            let map = kv
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect::<HashMap<_, _>>();
            let c = ExecutionConfig::from_env(&map).unwrap();
            let l = serde_json::to_string_pretty(&c).unwrap();
            let r = serde_json::to_string_pretty(&result).unwrap();
            assert_eq!(l, r);
        }
        t(
            [(
                OVERWRITE_FEEPAYER,
                "HJbqSuV94woJfyxFNnJyfQdACvvJYaNWsW1x6wmJ8kiq",
            )],
            ExecutionConfig {
                overwrite_feepayer: Some(WalletOrPubkey::Pubkey(pubkey!(
                    "HJbqSuV94woJfyxFNnJyfQdACvvJYaNWsW1x6wmJ8kiq"
                ))),
                ..<_>::default()
            },
        );
        t(
            [
                (COMPUTE_BUDGET, "auto"),
                (FALLBACK_COMPUTE_BUDGET, "500000"),
                (PRIORITY_FEE, "1000"),
                (SIMULATION_COMMITMENT_LEVEL, "confirmed"),
                (TX_COMMITMENT_LEVEL, "finalized"),
                (WAIT_COMMITMENT_LEVEL, "processed"),
            ],
            ExecutionConfig {
                compute_budget: InsertionBehavior::Auto,
                fallback_compute_budget: Some(500000),
                priority_fee: InsertionBehavior::Value(1000),
                simulation_commitment_level: CommitmentLevel::Confirmed,
                tx_commitment_level: CommitmentLevel::Finalized,
                wait_commitment_level: CommitmentLevel::Processed,
                ..<_>::default()
            },
        );
    }

    #[test]
    fn test_keypair_or_pubkey_keypair() {
        let keypair = Keypair::new();
        let x = WalletOrPubkey::Wallet(Wallet::Keypair(keypair.insecure_clone()));
        let value = value::to_value(&x).unwrap();
        assert_eq!(value, Value::B64(keypair.to_bytes()));
        assert_eq!(value::from_value::<WalletOrPubkey>(value).unwrap(), x);
    }

    #[test]
    fn test_keypair_or_pubkey_adapter() {
        let pubkey = Pubkey::new_unique();
        let x = WalletOrPubkey::Wallet(Wallet::Adapter { public_key: pubkey });
        let value = value::to_value(&x).unwrap();
        assert_eq!(
            value,
            Value::Map(value::map! {
                "public_key" => pubkey,
            })
        );
        assert_eq!(value::from_value::<WalletOrPubkey>(value).unwrap(), x);
    }

    #[test]
    fn test_keypair_or_pubkey_pubkey() {
        let pubkey = Pubkey::new_unique();
        let x = WalletOrPubkey::Pubkey(pubkey);
        let value = value::to_value(&x).unwrap();
        assert_eq!(value, Value::B32(pubkey.to_bytes()));
        assert_eq!(value::from_value::<WalletOrPubkey>(value).unwrap(), x);
    }

    #[test]
    fn test_wallet_keypair() {
        let keypair = Keypair::new();
        let x = Wallet::Keypair(keypair.insecure_clone());
        let value = value::to_value(&x).unwrap();
        assert_eq!(value, Value::B64(keypair.to_bytes()));
        assert_eq!(value::from_value::<Wallet>(value).unwrap(), x);
    }

    #[test]
    fn test_wallet_adapter() {
        let pubkey = Pubkey::new_unique();
        let x = Wallet::Adapter { public_key: pubkey };
        let value = value::to_value(&x).unwrap();
        assert_eq!(
            value,
            Value::Map(value::map! {
                "public_key" => pubkey,
            })
        );
        assert_eq!(value::from_value::<Wallet>(value).unwrap(), x);
    }

    #[test]
    fn test_instructions_bincode() {
        let instructions = Instructions {
            fee_payer: Pubkey::new_unique(),
            signers: [
                Wallet::Keypair(Keypair::new()),
                Wallet::Adapter {
                    public_key: Pubkey::new_unique(),
                },
            ]
            .into(),
            instructions: [transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1000)].into(),
            lookup_tables: Some([Pubkey::new_unique()].into()),
        };
        let data = bincode::encode_to_vec(&instructions, standard()).unwrap();
        let decoded: Instructions = bincode::decode_from_slice(&data, standard()).unwrap().0;
        dbg!(decoded);
    }
}
