use solana_sdk::pubkey::Pubkey;

pub(crate) const TOKEN: &str = "$$p";

pub type Target = Pubkey;

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

pub fn serialize<S>(p: &Target, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_newtype_struct(TOKEN, &crate::Bytes(p.as_ref()))
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = Target;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("pubkey, keypair, or bs58 string")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v.len() {
            32 => Ok(Pubkey::new_from_array(v.try_into().unwrap())),
            // see ed25519-dalek's Keypair
            64 => Ok(Pubkey::new_from_array(v[32..].try_into().unwrap())),
            l => Err(serde::de::Error::invalid_length(l, &"32 or 64")),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut buf = [0u8; 64];
        let size = bs58::decode(v).into(&mut buf).map_err(|_| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"pubkey or keypair encoded in bs58",
            )
        })?;
        self.visit_bytes(&buf[..size])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut buf = [0u8; 64];
        let mut size = 0;
        let mut iter_mut = buf.iter_mut();
        loop {
            match (seq.next_element()?, iter_mut.next()) {
                (Some(value), Some(ptr)) => {
                    size += 1;
                    *ptr = value;
                }
                (None, None) | (None, Some(_)) => break,
                (Some(_), None) => {
                    return Err(serde::de::Error::custom("array has more than 64 elements"));
                }
            }
        }
        self.visit_bytes(&buf[..size])
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
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signer;
    use solana_sdk::signer::keypair::Keypair;

    fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Pubkey {
        deserialize(d).unwrap()
    }

    #[test]
    fn test_deserialize_value() {
        let id = solana_sdk::feature_set::add_set_compute_unit_price_ix::id();
        assert_eq!(de(Value::String(id.to_string())), id);
        assert_eq!(de(Value::B32(id.to_bytes())), id);

        let k = Keypair::new();
        let pk = k.pubkey();
        assert_eq!(de(Value::B64(k.to_bytes())), pk);
    }

    #[test]
    fn test_serialize() {
        let id = solana_sdk::feature_set::add_set_compute_unit_price_ix::id();
        assert_eq!(
            serialize(&id, crate::ser::Serializer).unwrap(),
            Value::B32(id.to_bytes())
        );
    }
}
