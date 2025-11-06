use crate::{Value, value_type::Variant};
use base64::prelude::*;
use serde::de::VariantAccess;
use std::borrow::Cow;

pub struct TextRepr(Value);

impl From<TextRepr> for Value {
    fn from(v: TextRepr) -> Value {
        v.0
    }
}

struct EnumVisitor;

impl<'de> serde::de::Visitor<'de> for EnumVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("any valid value")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        let (ty, a) = data.variant::<Variant>()?;
        match ty {
            Variant::Null => {
                let num = a.newtype_variant::<u64>()?;
                if num != 0 {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(num),
                        &"0",
                    ));
                }
                Ok(Value::Null)
            }
            Variant::String => Ok(Value::String(a.newtype_variant()?)),
            Variant::Bool => Ok(Value::Bool(a.newtype_variant()?)),
            Variant::U64 => Ok(Value::U64(number_from_str(a)?)),
            Variant::I64 => Ok(Value::I64(number_from_str(a)?)),
            Variant::F64 => Ok(Value::F64(number_from_str(a)?)),
            Variant::Decimal => Ok(Value::Decimal(number_from_str(a)?)),
            Variant::I128 => Ok(Value::I128(number_from_str(a)?)),
            Variant::U128 => Ok(Value::U128(number_from_str(a)?)),
            Variant::B32 => Ok(Value::B32(b58_str_32(a)?)),
            Variant::B64 => Ok(Value::B64(b58_str_64(a)?)),
            Variant::Bytes => Ok(Value::Bytes(b64_str(a)?)),
            Variant::Array => Ok(Value::Array(a.newtype_variant::<Array>()?.0)),
            Variant::Map => Ok(Value::Map(a.newtype_variant::<Map>()?.0)),
        }
    }
}

struct MapVisitor;

impl<'de> serde::de::Visitor<'de> for MapVisitor {
    type Value = crate::Map;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("map")
    }

    fn visit_map<A>(self, mut a: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut map = crate::Map::new();
        if let Some(len) = a.size_hint() {
            map.reserve(len);
        }
        while let Some((k, v)) = a.next_entry::<crate::Key, TextRepr>()? {
            map.insert(k, v.into());
        }
        Ok(map)
    }
}

struct Map(crate::Map);

impl<'de> serde::Deserialize<'de> for Map {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Map(d.deserialize_map(MapVisitor)?))
    }
}

struct ArrayVisitor;

impl<'de> serde::de::Visitor<'de> for ArrayVisitor {
    type Value = Vec<Value>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("array")
    }

    fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        if let Some(len) = a.size_hint() {
            vec.reserve(len);
        }
        while let Some(v) = a.next_element::<TextRepr>()? {
            vec.push(v.into());
        }
        Ok(vec)
    }
}

struct Array(Vec<Value>);

impl<'de> serde::Deserialize<'de> for Array {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Array(d.deserialize_seq(ArrayVisitor)?))
    }
}

fn number_from_str<'de, A, T>(a: A) -> Result<T, A::Error>
where
    A: VariantAccess<'de>,
    T: std::str::FromStr,
{
    let s = a.newtype_variant::<Cow<'_, str>>()?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom(format!("invalid number: {s}")))
}

fn b58_str_32<'de, A>(a: A) -> Result<[u8; 32], A::Error>
where
    A: VariantAccess<'de>,
{
    let s = a.newtype_variant::<Cow<'_, str>>()?;
    let mut data = [0u8; 32];
    five8::decode_32(s.as_bytes(), &mut data)
        .map_err(|_| serde::de::Error::custom("invalid base58"))?;
    Ok(data)
}

fn b58_str_64<'de, A>(a: A) -> Result<[u8; 64], A::Error>
where
    A: VariantAccess<'de>,
{
    let s = a.newtype_variant::<Cow<'_, str>>()?;
    let mut data = [0u8; 64];
    five8::decode_64(s.as_bytes(), &mut data)
        .map_err(|_| serde::de::Error::custom("invalid base58"))?;
    Ok(data)
}

fn b64_str<'de, A>(a: A) -> Result<bytes::Bytes, A::Error>
where
    A: VariantAccess<'de>,
{
    let s = a.newtype_variant::<Cow<'_, str>>()?;
    BASE64_STANDARD
        .decode(s.as_bytes())
        .map_err(|_| serde::de::Error::custom("invalid base64"))
        .map(Into::into)
}

impl<'de> serde::Deserialize<'de> for TextRepr {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = d.deserialize_enum(crate::TOKEN, crate::value_type::keys::ALL, EnumVisitor)?;
        Ok(TextRepr(value))
    }
}
