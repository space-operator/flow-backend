use crate::with::AsKeypair;
use solana_keypair::Keypair;

type Target = Keypair;

type As = AsKeypair;

pub mod opt {
    use serde_with::{DeserializeAs, SerializeAs};

    pub fn serialize<S>(sig: &Option<super::Target>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Option::<super::As>::serialize_as(sig, s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<super::Target>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<super::As>::deserialize_as(d)
    }
}

pub fn serialize<S>(p: &Target, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    As::serialize(p, s)
}

pub fn deserialize<'de, D>(d: D) -> Result<Target, D::Error>
where
    D: serde::Deserializer<'de>,
{
    As::deserialize(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use solana_sdk::signer::keypair::Keypair;

    fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Keypair {
        deserialize(d).unwrap()
    }

    #[test]
    fn test_deserialize_value() {
        let k = Keypair::new();
        assert_eq!(de(Value::B64(k.to_bytes())), k);
        assert_eq!(de(Value::String(k.to_base58_string())), k);
    }

    #[test]
    fn test_serialize() {
        let k = Keypair::new();
        assert_eq!(
            serialize(&k, crate::ser::Serializer).unwrap(),
            Value::B64(k.to_bytes()),
        )
    }

    #[test]
    fn test_enum() {
        let key = Keypair::new();

        #[derive(serde::Deserialize, PartialEq, Debug)]
        #[serde(untagged)]
        pub enum UntaggedEnum {
            PrivateKey {
                #[serde(with = "super")]
                private_key: Keypair,
            },
            Seed {
                #[serde(default)]
                seed: String,
                #[serde(default)]
                passphrase: String,
            },
        }
        assert_eq!(
            crate::from_map::<UntaggedEnum>(crate::Map::from([(
                "private_key".to_owned(),
                Value::B64(key.to_bytes())
            )]))
            .unwrap(),
            UntaggedEnum::PrivateKey { private_key: key }
        );
    }
}
