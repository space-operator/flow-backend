use solana_sdk::signer::keypair::Keypair;

pub(crate) const TOKEN: &str = "$$k";

pub type Target = Keypair;

pub mod opt {
    pub fn serialize<S>(sig: &Option<super::Target>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match sig {
            Some(sig) => super::serialize(sig, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<super::Target>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_option(crate::OptionVisitor(super::Visitor))
    }
}

pub fn serialize<S>(k: &Target, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_newtype_struct(TOKEN, &crate::Bytes(&k.to_bytes()))
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = Keypair;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("keypair")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Keypair::from_bytes(v).map_err(|_| serde::de::Error::invalid_length(v.len(), &"64"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut buf = [0u8; 64];
        let size = bs58::decode(v).into(&mut buf).map_err(|_| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"keypair encoded in bs58",
            )
        })?;
        self.visit_bytes(&buf[..size])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut buf = [0u8; 64];
        let mut iter_mut = buf.iter_mut();
        loop {
            match (seq.next_element()?, iter_mut.next()) {
                (Some(value), Some(ptr)) => *ptr = value,
                (None, None) => break,
                _ => return Err(serde::de::Error::custom("expected array of 64 elements")),
            }
        }
        Keypair::from_bytes(&buf).map_err(|_| serde::de::Error::custom("invalid keypair"))
    }

    fn visit_newtype_struct<D>(self, d: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_any(self)
    }
}

pub fn deserialize<'de, D>(d: D) -> Result<Target, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_newtype_struct(TOKEN, Visitor)
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
