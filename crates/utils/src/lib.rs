use thiserror::Error as ThisError;

pub mod actix_service;
pub mod address_book;
pub mod serde_base64;
pub mod serde_bs58;

pub struct B58<const N: usize>(pub [u8; N]);

#[derive(ThisError, Debug)]
pub enum Bs58Error {
    #[error(transparent)]
    Decode(#[from] bs58::decode::Error),
    #[error("invalid length, expected: {}, got: {}", expected, got)]
    Size { expected: usize, got: usize },
}

impl<const N: usize> std::str::FromStr for B58<N> {
    type Err = Bs58Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut buf = [0u8; N];
        let size = bs58::decode(s).into(&mut buf)?;
        if size != N {
            return Err(Bs58Error::Size {
                expected: N,
                got: size,
            });
        }
        Ok(Self(buf))
    }
}

pub fn bs58_decode<const N: usize>(s: &str) -> Result<[u8; N], Bs58Error> {
    Ok(s.parse::<B58<N>>()?.0)
}

pub struct OptionVisitor<V>(pub(crate) V);

impl<'de, V> serde::de::Visitor<'de> for OptionVisitor<V>
where
    V: serde::de::Visitor<'de>,
{
    type Value = Option<V::Value>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("optional ")?;
        self.0.expecting(formatter)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_any(self.0).map(Some)
    }
}
