pub fn serialize<const N: usize, S>(t: &[u8; N], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&bs58::encode(t).into_string())
}

struct Visitor<const N: usize>;

impl<'de, const N: usize> serde::de::Visitor<'de> for Visitor<N> {
    type Value = [u8; N];

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("base58")
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
    d.deserialize_str(Visitor::<S>)
}

pub mod opt {
    struct Bs58<'a>(&'a [u8]);
    impl<'a> serde::Serialize for Bs58<'a> {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            s.serialize_str(&bs58::encode(self.0).into_string())
        }
    }

    pub fn serialize<const N: usize, S>(sig: &Option<[u8; N]>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match sig {
            Some(sig) => s.serialize_some(&Bs58(sig.as_slice())),
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
