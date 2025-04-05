//! [serde_with](https://docs.rs/serde_with/latest/serde_with/) helpers.

use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess},
};
use serde_with::serde_conv;
use std::{borrow::Cow, convert::Infallible};
use std::{mem::MaybeUninit, ops::ControlFlow};

pub use decimal::AsDecimal;
#[cfg(feature = "solana-keypair")]
pub use keypair::AsKeypair;
#[cfg(feature = "solana-pubkey")]
pub use pubkey::AsPubkey;
#[cfg(feature = "solana-signature")]
pub use signature::AsSignature;

fn try_from_fn_erased<T: Copy, E>(
    buffer: &mut [MaybeUninit<T>],
    mut generator: impl FnMut(usize) -> Result<T, E>,
) -> ControlFlow<E> {
    for (i, elem) in buffer.iter_mut().enumerate() {
        let item = match generator(i) {
            Ok(item) => item,
            Err(error) => return ControlFlow::Break(error),
        };
        elem.write(item);
    }

    ControlFlow::Continue(())
}

fn try_from_fn<const N: usize, T: Copy, E, F>(cb: F) -> Result<[T; N], E>
where
    F: FnMut(usize) -> Result<T, E>,
{
    let mut array = [const { MaybeUninit::uninit() }; N];
    match try_from_fn_erased(&mut array, cb) {
        ControlFlow::Break(error) => Err(error),
        ControlFlow::Continue(()) => Ok(array.map(|uninit| unsafe { uninit.assume_init() })),
    }
}

#[cfg(feature = "solana-pubkey")]
pub(crate) mod pubkey {
    use std::marker::PhantomData;

    use super::*;
    use five8::BASE58_ENCODED_32_MAX_LEN;
    use solana_pubkey::Pubkey;

    struct CustomPubkey<'a>(Cow<'a, Pubkey>);

    pub(crate) const TOKEN: &str = "$$p";

    impl Serialize for CustomPubkey<'_> {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            s.serialize_newtype_struct(TOKEN, &crate::Bytes((*self.0).as_ref()))
        }
    }

    impl<'de> Deserialize<'de> for CustomPubkey<'_> {
        fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_newtype_struct(TOKEN, Visitor { map: true })
                .map(|pk| CustomPubkey(Cow::Owned(pk)))
        }
    }

    struct Visitor {
        map: bool,
    }

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Pubkey;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            if self.map {
                formatter.write_str("pubkey, keypair, base58 string, or adapter wallet")
            } else {
                formatter.write_str("pubkey, keypair, or base58 string")
            }
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
            if v.len() > BASE58_ENCODED_32_MAX_LEN {
                let mut buf = [0u8; 64];
                five8::decode_64(v, &mut buf).map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &"pubkey or keypair encoded in bs58",
                    )
                })?;
                Ok(Pubkey::new_from_array(buf[32..].try_into().unwrap()))
            } else {
                let mut buf = [0u8; 32];
                five8::decode_32(v, &mut buf).map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &"pubkey or keypair encoded in bs58",
                    )
                })?;
                Ok(Pubkey::new_from_array(buf))
            }
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let hint = seq.size_hint();
            match hint {
                Some(n) => {
                    if n == 32 {
                        let buffer: [u8; 32] = try_from_fn(|i| {
                            seq.next_element()?
                                .ok_or_else(|| de::Error::invalid_length(i, &"32"))
                        })?;
                        Ok(Pubkey::new_from_array(buffer))
                    } else if n == 64 {
                        for _ in 0..32 {
                            seq.next_element::<u8>()?;
                        }
                        let buffer: [u8; 32] = try_from_fn(|i| {
                            seq.next_element()?
                                .ok_or_else(|| de::Error::invalid_length(i + 32, &"64"))
                        })?;
                        Ok(Pubkey::new_from_array(buffer))
                    } else {
                        Err(de::Error::invalid_length(n, &"32 or 64"))
                    }
                }
                None => {
                    let buffer: [u8; 32] = try_from_fn(|i| {
                        seq.next_element()?
                            .ok_or_else(|| de::Error::invalid_length(i, &"32"))
                    })?;
                    let next = seq.next_element::<u8>()?;
                    if let Some(x) = next {
                        let mut result = [0u8; 32];
                        result[0] = x;
                        let buffer: [u8; 31] = try_from_fn(|i| {
                            seq.next_element()?
                                .ok_or_else(|| de::Error::invalid_length(i, &"64"))
                        })?;
                        result[1..].copy_from_slice(&buffer);
                        Ok(Pubkey::new_from_array(result))
                    } else {
                        Ok(Pubkey::new_from_array(buffer))
                    }
                }
            }
        }

        fn visit_newtype_struct<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_any(self)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            if self.map {
                map.next_key::<Const<public_key>>()?;
                let value = map.next_value::<CustomPubkeyNoMap>()?;
                Ok(value.0)
            } else {
                Err(de::Error::invalid_type(de::Unexpected::Map, &self))
            }
        }
    }

    struct CustomPubkeyNoMap(Pubkey);

    impl<'de> Deserialize<'de> for CustomPubkeyNoMap {
        fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            d.deserialize_any(Visitor { map: false })
                .map(CustomPubkeyNoMap)
        }
    }

    #[allow(non_camel_case_types)]
    struct public_key;

    impl Key for public_key {
        const KEY: &'static str = "public_key";
        fn new() -> Self {
            Self
        }
    }

    trait Key {
        const KEY: &'static str;
        fn new() -> Self;
    }

    struct Const<K>(K);

    impl<'de, K> Deserialize<'de> for Const<K>
    where
        K: Key,
    {
        fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            d.deserialize_str(StrVisitor::<K>(PhantomData))
        }
    }

    struct StrVisitor<K: Key>(PhantomData<fn() -> K>);

    impl<K: Key> de::Visitor<'_> for StrVisitor<K> {
        type Value = Const<K>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str(K::KEY)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v == K::KEY {
                Ok(Const(K::new()))
            } else {
                Err(de::Error::invalid_value(de::Unexpected::Str(v), &K::KEY))
            }
        }
    }

    fn to_custom_pubkey(pk: &Pubkey) -> CustomPubkey<'_> {
        CustomPubkey(Cow::Borrowed(pk))
    }
    fn from_custom_pubkey(pk: CustomPubkey<'static>) -> Result<Pubkey, Infallible> {
        Ok(pk.0.into_owned())
    }
    serde_conv!(pub AsPubkey, Pubkey, to_custom_pubkey, from_custom_pubkey);

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::Value;
        use serde_with::{DeserializeAs, SerializeAs};
        use solana_keypair::Keypair;
        use solana_signer::Signer;

        #[test]
        fn test_pubkey() {
            let key = Pubkey::new_unique();
            let value = AsPubkey::serialize_as(&key, crate::ser::Serializer).unwrap();
            assert!(matches!(value, Value::B32(_)));
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::Map(crate::map! { "public_key" => key });
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::String(key.to_string());
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::Array(key.to_bytes().map(Value::from).to_vec());
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let keypair = Keypair::new();
            let key = keypair.pubkey();
            let value = Value::B64(keypair.to_bytes());
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::String(keypair.to_base58_string());
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::Array(keypair.to_bytes().map(Value::from).to_vec());
            let de_key = AsPubkey::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);
        }
    }
}

#[cfg(feature = "solana-signature")]
pub(crate) mod signature {
    use super::*;
    use solana_signature::Signature;

    struct CustomSignature<'a>(Cow<'a, Signature>);

    pub(crate) const TOKEN: &str = "$$s";

    impl Serialize for CustomSignature<'_> {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            s.serialize_newtype_struct(TOKEN, &crate::Bytes((*self.0).as_ref()))
        }
    }

    impl<'de> Deserialize<'de> for CustomSignature<'_> {
        fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_newtype_struct(TOKEN, Visitor)
                .map(|pk| CustomSignature(Cow::Owned(pk)))
        }
    }

    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Signature;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("signature or bs58 string")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let buffer: [u8; 64] = v
                .try_into()
                .map_err(|_| de::Error::invalid_length(v.len(), &"64"))?;
            Ok(Signature::from(buffer))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut buffer = [0u8; 64];
            five8::decode_64(v, &mut buffer).map_err(de::Error::custom)?;
            Ok(Signature::from(buffer))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let buffer: [u8; 64] = try_from_fn(|i| {
                seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(i, &"64"))
            })?;

            Ok(Signature::from(buffer))
        }

        fn visit_newtype_struct<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_any(self)
        }
    }

    fn to_custom_signature(pk: &Signature) -> CustomSignature<'_> {
        CustomSignature(Cow::Borrowed(pk))
    }
    fn from_custom_signature(pk: CustomSignature<'static>) -> Result<Signature, Infallible> {
        Ok(pk.0.into_owned())
    }
    serde_conv!(pub AsSignature, Signature, to_custom_signature, from_custom_signature);

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::Value;
        use serde_with::{DeserializeAs, SerializeAs};
        use solana_signature::Signature;

        #[test]
        fn test_signature() {
            let sig = Signature::default();
            let value = AsSignature::serialize_as(&sig, crate::ser::Serializer).unwrap();
            assert!(matches!(value, Value::B64(_)));
            let de_sig = AsSignature::deserialize_as(value).unwrap();
            assert_eq!(sig, de_sig);

            let value = Value::String(sig.to_string());
            let de_sig = AsSignature::deserialize_as(value).unwrap();
            assert_eq!(sig, de_sig);

            let value = Value::Array(
                sig.as_ref()
                    .iter()
                    .map(|i| Value::from(*i))
                    .collect::<Vec<_>>(),
            );
            let de_sig = AsSignature::deserialize_as(value).unwrap();
            assert_eq!(sig, de_sig);
        }
    }
}

#[cfg(feature = "solana-keypair")]
pub(crate) mod keypair {
    use super::*;
    use solana_keypair::Keypair;

    struct CustomKeypair([u8; 64]);

    pub(crate) const TOKEN: &str = "$$k";

    impl Serialize for CustomKeypair {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            s.serialize_newtype_struct(TOKEN, &crate::Bytes(&self.0))
        }
    }

    impl<'de> Deserialize<'de> for CustomKeypair {
        fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_newtype_struct(TOKEN, Visitor)
        }
    }

    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = CustomKeypair;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("keypair or bs58 string")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let buffer: [u8; 64] = v
                .try_into()
                .map_err(|_| de::Error::invalid_length(v.len(), &"64"))?;
            Ok(CustomKeypair(buffer))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut buffer = [0u8; 64];
            five8::decode_64(v, &mut buffer).map_err(de::Error::custom)?;
            Ok(CustomKeypair(buffer))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let buffer: [u8; 64] = try_from_fn(|i| {
                seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(i, &"64"))
            })?;

            Ok(CustomKeypair(buffer))
        }

        fn visit_newtype_struct<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_any(self)
        }
    }

    fn to_custom_keypair(k: &'_ Keypair) -> CustomKeypair {
        CustomKeypair(k.to_bytes())
    }
    fn from_custom_keypair(k: CustomKeypair) -> Result<Keypair, String> {
        Keypair::from_bytes(&k.0).map_err(|error| error.to_string())
    }
    serde_conv!(pub AsKeypair, Keypair, to_custom_keypair, from_custom_keypair);

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::Value;
        use serde_with::{DeserializeAs, SerializeAs};

        #[test]
        fn test_keypair() {
            let key = Keypair::new();
            let value = AsKeypair::serialize_as(&key, crate::ser::Serializer).unwrap();
            assert!(matches!(value, Value::B64(_)));
            let de_key = AsKeypair::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::String(key.to_base58_string());
            let de_key = AsKeypair::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);

            let value = Value::Array(key.to_bytes().map(Value::from).to_vec());
            let de_key = AsKeypair::deserialize_as(value).unwrap();
            assert_eq!(key, de_key);
        }
    }
}

pub(crate) mod decimal {
    use super::*;
    use rust_decimal::Decimal;

    struct CustomDecimal<'a>(Cow<'a, Decimal>);

    pub(crate) const TOKEN: &str = "$$d";

    impl Serialize for CustomDecimal<'_> {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            s.serialize_newtype_struct(TOKEN, &crate::Bytes(&(*self.0).serialize()))
        }
    }

    impl<'de> Deserialize<'de> for CustomDecimal<'_> {
        fn deserialize<D>(d: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            d.deserialize_newtype_struct(TOKEN, Visitor)
                .map(|d| CustomDecimal(Cow::Owned(d)))
        }
    }

    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Decimal;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("decimal")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let buf: [u8; 16] = v
                .try_into()
                .map_err(|_| de::Error::invalid_length(v.len(), &"16"))?;
            Ok(Decimal::deserialize(buf))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Decimal::from(v))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Decimal::from(v))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // TODO: this is lossy
            Decimal::try_from(v).map_err(serde::de::Error::custom)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let v = v.trim();
            if v.bytes().any(|c| c == b'e' || c == b'E') {
                Decimal::from_scientific(v).map_err(serde::de::Error::custom)
            } else {
                v.parse().map_err(serde::de::Error::custom)
            }
        }

        fn visit_newtype_struct<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            d.deserialize_any(self)
        }
    }

    fn to_custom_decimal(d: &Decimal) -> CustomDecimal<'_> {
        CustomDecimal(Cow::Borrowed(d))
    }
    fn from_custom_decimal(d: CustomDecimal<'static>) -> Result<Decimal, Infallible> {
        Ok(d.0.into_owned())
    }
    serde_conv!(pub AsDecimal, Decimal, to_custom_decimal, from_custom_decimal);

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::Value;
        use rust_decimal_macros::dec;
        use serde_with::{DeserializeAs, SerializeAs};

        fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Decimal {
            AsDecimal::deserialize_as(d).unwrap()
        }

        #[test]
        fn test_decimal() {
            assert_eq!(
                AsDecimal::serialize_as(&Decimal::MAX, crate::ser::Serializer).unwrap(),
                Value::Decimal(Decimal::MAX)
            );
            assert_eq!(de(Value::U64(100)), dec!(100));
            assert_eq!(de(Value::I64(-1)), dec!(-1));
            assert_eq!(de(Value::Decimal(Decimal::MAX)), Decimal::MAX);
            assert_eq!(de(Value::F64(1231.2221)), dec!(1231.2221));
            assert_eq!(de(Value::String("1234.0".to_owned())), dec!(1234));
            assert_eq!(de(Value::String("  1234.0".to_owned())), dec!(1234));
            assert_eq!(de(Value::String("1e5".to_owned())), dec!(100000));
            assert_eq!(de(Value::String("  1e5".to_owned())), dec!(100000));
        }
    }
}
