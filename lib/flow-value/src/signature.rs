use crate::with::AsSignature;
use solana_signature::Signature;

type Target = Signature;

type As = AsSignature;

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

    fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Signature {
        deserialize(d).unwrap()
    }

    #[test]
    fn test_deserialize_value() {
        let s = Signature::default();
        assert_eq!(de(Value::B64(s.into())), s);
        assert_eq!(de(Value::String(s.to_string())), s);
    }

    #[test]
    fn test_serialize() {
        let s = Signature::default();
        assert_eq!(
            serialize(&s, crate::ser::Serializer).unwrap(),
            Value::B64(s.into())
        );
    }
}
