pub fn serialize<const N: usize, S>(t: &[u8; N], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&bs58::encode(t).into_string())
}

pub struct Visitor<const N: usize>;

impl<'de, const N: usize> serde::de::Visitor<'de> for Visitor<N> {
    type Value = [u8; N];

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("base58 public key")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut pk = [0u8; N];
        let size = bs58::decode(v)
            .into(&mut pk)
            .map_err(|_| serde::de::Error::custom("invalid base58"))?;
        if size != N {
            return Err(serde::de::Error::custom("invalid base58"));
        }
        Ok(pk)
    }
}

pub fn deserialize<'de, const S: usize, D>(d: D) -> Result<[u8; S], D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor<const N: usize>;

    impl<'de, const N: usize> serde::de::Visitor<'de> for Visitor<N> {
        type Value = [u8; N];

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("base58 public key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut pk = [0u8; N];
            let size = bs58::decode(v)
                .into(&mut pk)
                .map_err(|_| serde::de::Error::custom("invalid base58"))?;
            if size != N {
                return Err(serde::de::Error::custom("invalid base58"));
            }
            Ok(pk)
        }
    }

    d.deserialize_str(Visitor::<S>)
}

pub mod opt {
    pub fn serialize<const N: usize, S>(sig: &Option<[u8; N]>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match sig {
            Some(sig) => super::serialize(sig, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, const N: usize, D>(d: D) -> Result<Option<[u8; N]>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_option(crate::OptionVisitor(super::Visitor))
    }
}
