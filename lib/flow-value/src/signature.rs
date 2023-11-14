use solana_sdk::signature::Signature;

pub(crate) const TOKEN: &str = "$$s";

pub type Target = Signature;

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

pub fn serialize<S>(sig: &Target, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_newtype_struct(TOKEN, &crate::Bytes(sig.as_ref()))
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = Target;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("keypair, or bs58 string")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match <[u8; 64]>::try_from(v) {
            Ok(x) => Ok(Signature::from(x)),
            _ => Err(serde::de::Error::invalid_length(v.len(), &"64")),
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
                &"signature encoded in bs58",
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
        Ok(Signature::from(buf))
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

    fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Signature {
        deserialize(d).unwrap()
    }

    #[test]
    fn test_deserialize_value() {
        let s = Signature::new_unique();
        assert_eq!(de(Value::B64(s.into())), s);
        assert_eq!(de(Value::String(s.to_string())), s);
    }

    #[test]
    fn test_serialize() {
        let s = Signature::new_unique();
        assert_eq!(
            serialize(&s, crate::ser::Serializer).unwrap(),
            Value::B64(s.into())
        );
    }
}
