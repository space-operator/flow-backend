use base64::prelude::*;

pub fn serialize<S>(t: &bytes::Bytes, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&BASE64_STANDARD.encode(t))
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = bytes::Bytes;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("base64")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(BASE64_STANDARD
            .decode(v)
            .map_err(|_| serde::de::Error::custom("invalid base64"))?
            .into())
    }
}

pub fn deserialize<'de, D>(d: D) -> Result<bytes::Bytes, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_str(Visitor)
}

pub mod opt {
    pub fn serialize<S>(sig: &Option<bytes::Bytes>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match sig {
            Some(sig) => super::serialize(sig, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<bytes::Bytes>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_option(crate::OptionVisitor(super::Visitor))
    }
}
