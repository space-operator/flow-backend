use crate::{value_type::Variant, Error, Map, Value};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::de::value::SeqDeserializer;

pub(crate) mod const_bytes;
mod de_enum;
mod de_struct;
mod text_repr;

use const_bytes::ConstBytes;
use de_enum::{EnumDeserializer, ValueEnumAccess};
use de_struct::MapDeserializer;

struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("any valid value")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        use serde::de::VariantAccess;
        let (ty, a) = data.variant::<Variant>()?;
        match ty {
            Variant::Null => Ok(Value::Null),
            Variant::String => Ok(Value::String(a.newtype_variant()?)),
            Variant::Bool => Ok(Value::Bool(a.newtype_variant()?)),
            Variant::U64 => Ok(Value::U64(a.newtype_variant()?)),
            Variant::I64 => Ok(Value::I64(a.newtype_variant()?)),
            Variant::F64 => Ok(Value::F64(a.newtype_variant()?)),
            Variant::Decimal => Ok(Value::Decimal(Decimal::deserialize(
                a.newtype_variant::<ConstBytes<16>>()?.0,
            ))),
            Variant::I128 => Ok(Value::I128(a.newtype_variant()?)),
            Variant::U128 => Ok(Value::U128(a.newtype_variant()?)),
            Variant::B32 => Ok(Value::B32(a.newtype_variant::<ConstBytes<32>>()?.0)),
            Variant::B64 => Ok(Value::B64(a.newtype_variant::<ConstBytes<64>>()?.0)),
            Variant::Bytes => Ok(Value::Bytes(a.newtype_variant()?)),
            Variant::Array => Ok(Value::Array(a.newtype_variant()?)),
            Variant::Map => Ok(Value::Map(a.newtype_variant()?)),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    /// Turn any `Deserializer` into `Value`, intended to be used
    /// with `Value as Deserializer`.
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if d.is_human_readable() {
            text_repr::TextRepr::deserialize(d).map(Into::into)
        } else {
            d.deserialize_enum(crate::TOKEN, crate::value_type::keys::ALL, ValueVisitor)
        }
    }
}

impl<'de> serde::Deserializer<'de> for Value {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::String(s) => visitor.visit_string(s),
            Value::Bool(b) => visitor.visit_bool(b),
            Value::I64(i) => visitor.visit_i64(i),
            Value::U64(u) => visitor.visit_u64(u),
            Value::F64(f) => visitor.visit_f64(f),
            Value::Decimal(d) => visit_decimal(d, visitor),
            Value::I128(i) => visitor.visit_i128(i),
            Value::U128(u) => visitor.visit_u128(u),
            Value::Array(array) => visit_array(array, visitor),
            Value::Map(map) => visit_map(map, visitor),
            Value::B32(x) => visit_bytes(&x, visitor),
            Value::B64(x) => visit_bytes(&x, visitor),
            Value::Bytes(x) => visit_bytes(&x, visitor),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match name {
            crate::decimal::TOKEN => match self {
                Value::Decimal(d) => visitor.visit_bytes(&d.serialize()),
                Value::I64(i) => visitor.visit_i64(i),
                Value::U64(u) => visitor.visit_u64(u),
                Value::F64(f) => visitor.visit_f64(f),
                Value::String(s) => visitor.visit_string(s),
                _ => Err(serde::de::Error::invalid_type(
                    self.unexpected(),
                    &"decimal",
                )),
            },
            #[cfg(feature = "solana")]
            crate::keypair::TOKEN | crate::signature::TOKEN => match self {
                Value::B64(b) => visitor.visit_bytes(&b),
                Value::Bytes(b) if b.len() == 64 => visitor.visit_bytes(&b),
                Value::String(s) => visitor.visit_str(&s),
                _ => Err(serde::de::Error::invalid_type(
                    self.unexpected(),
                    &"bytes or base58 string",
                )),
            },
            #[cfg(feature = "solana")]
            crate::pubkey::TOKEN => match self {
                Value::B32(b) => visitor.visit_bytes(&b),
                Value::B64(b) => visitor.visit_bytes(&b[32..]),
                Value::Bytes(b) if b.len() == 32 => visitor.visit_bytes(&b),
                Value::Bytes(b) if b.len() == 64 => visitor.visit_bytes(&b[32..]),
                Value::String(s) => visitor.visit_str(&s),
                _ => Err(serde::de::Error::invalid_type(
                    self.unexpected(),
                    &"bytes or base58 string",
                )),
            },
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if name == crate::TOKEN {
            visitor.visit_enum(ValueEnumAccess(self))
        } else {
            let (variant, value) = match self {
                Value::Map(value) => {
                    let mut iter = value.into_iter();
                    let (variant, value) = match iter.next() {
                        Some(v) => v,
                        None => {
                            return Err(serde::de::Error::invalid_value(
                                serde::de::Unexpected::Map,
                                &"map with a single key",
                            ));
                        }
                    };
                    if iter.next().is_some() {
                        return Err(serde::de::Error::invalid_value(
                            serde::de::Unexpected::Map,
                            &"map with a single key",
                        ));
                    }
                    (variant, Some(value))
                }
                Value::String(variant) => (variant, None),
                other => {
                    return Err(serde::de::Error::invalid_type(
                        other.unexpected(),
                        &"string or map",
                    ));
                }
            };

            visitor.visit_enum(EnumDeserializer { variant, value })
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Decimal(v) => visitor.visit_bytes(&v.serialize()),
            Value::B32(v) => visitor.visit_bytes(&v),
            Value::B64(v) => visitor.visit_bytes(&v),
            Value::Bytes(v) => visitor.visit_bytes(&v),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        unit unit_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

fn visit_decimal<'de, V>(mut d: Decimal, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    d.normalize_assign();
    if d.scale() == 0 {
        if d.is_sign_negative() {
            if let Some(i) = d.to_i64() {
                return visitor.visit_i64(i);
            }
        } else if let Some(u) = d.to_u64() {
            return visitor.visit_u64(u);
        }
    }

    // this is lossy
    if let Some(f) = d.to_f64() {
        return visitor.visit_f64(f);
    }

    // I think to_f64 never fails, so this might be unreachable
    visitor.visit_string(d.to_string())
}

impl Value {
    pub(self) fn unexpected(&self) -> serde::de::Unexpected {
        use serde::de::Unexpected;
        match self {
            Value::Null => Unexpected::Unit,
            Value::String(s) => Unexpected::Str(s),
            Value::Bool(b) => Unexpected::Bool(*b),
            Value::I64(i) => Unexpected::Signed(*i),
            Value::U64(u) => Unexpected::Unsigned(*u),
            Value::F64(f) => Unexpected::Float(*f),
            Value::Decimal(_) => Unexpected::Other("decimal"),
            Value::I128(_) => Unexpected::Other("i128"),
            Value::U128(_) => Unexpected::Other("u128"),
            Value::Array(_) => Unexpected::Seq,
            Value::Map(_) => Unexpected::Map,
            Value::B32(_) => Unexpected::Other("[u8; 32]"),
            Value::B64(_) => Unexpected::Other("[u8; 64]"),
            Value::Bytes(_) => Unexpected::Other("bytes"),
        }
    }
}

fn visit_array<'de, V>(array: Vec<Value>, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    let mut deserializer = SeqDeserializer::<_, Error>::new(array.into_iter());
    let seq = visitor.visit_seq(&mut deserializer)?;
    deserializer.end()?;
    Ok(seq)
}

fn visit_bytes<'de, V>(b: &[u8], visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    let mut deserializer = SeqDeserializer::<_, Error>::new(b.iter().cloned());
    let seq = visitor.visit_seq(&mut deserializer)?;
    deserializer.end()?;
    Ok(seq)
}

impl<'de> serde::de::IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

fn visit_map<'de, V>(object: Map, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    let len = object.len();
    let mut deserializer = MapDeserializer::new(object);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(serde::de::Error::invalid_length(
            len,
            &"fewer elements in map",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde::Deserialize;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn value_to_value() {
        fn t(v: Value) {
            assert_eq!(Value::deserialize(v.clone()).unwrap(), v)
        }
        t(Value::Null);
        t(Value::I64(0i64));
        t(Value::String(String::new()));
        t(Value::Bool(false));
        t(Value::U64(0));
        t(Value::I64(0));
        t(Value::F64(0.0));
        t(Value::Decimal(Decimal::MAX));
        t(Value::I128(0));
        t(Value::U128(0));
        t(Value::B32([0u8; 32]));
        t(Value::B64([0u8; 64]));
        t(Value::Bytes(bytes::Bytes::from_static(
            "something".as_bytes(),
        )));
        t(Value::Array(Vec::new()));
        t(Value::Map(Map::new()));
    }

    fn de<T: serde::de::DeserializeOwned>(v: Value) -> T {
        T::deserialize(v).unwrap()
    }

    #[test]
    fn test_primitive() {
        assert_eq!(de::<u8>(Value::U64(0)), 0u8);
        assert_eq!(de::<i8>(Value::U64(0)), 0i8);
        assert_eq!(de::<u8>(Value::I64(1)), 1u8);
        assert_eq!(de::<i8>(Value::I64(-1)), -1i8);
        assert_eq!(de::<f32>(Value::F64(0.0)), 0f32);
        assert!(!de::<bool>(Value::Bool(false)));
        assert_eq!(de::<String>(Value::String("abc".to_owned())), "abc");
        assert_eq!(de::<f32>(Value::I64(1)), 1f32);
    }

    #[test]
    fn test_option() {
        assert_eq!(de::<Option<u32>>(Value::U64(0)), Some(0));
        assert_eq!(de::<Option<()>>(Value::Null), None);
        assert_eq!(de::<Option<Option<u32>>>(Value::U64(0)), Some(Some(0)));
    }

    #[test]
    fn test_array() {
        assert_eq!(
            de::<Vec<u32>>(Value::Array([Value::U64(0), Value::U64(1)].to_vec())),
            vec![0, 1],
        );

        assert_eq!(
            de::<(u32, f32, Option<u64>, (i32, i32), Vec<String>)>(Value::Array(
                [
                    Value::U64(0),
                    Value::F64(0.1),
                    Value::Null,
                    Value::Array([Value::I64(1), Value::I64(2),].to_vec()),
                    Value::Array([Value::String("hello".to_owned())].to_vec()),
                ]
                .to_vec()
            )),
            (0u32, 0.1f32, None, (1, 2), ["hello".to_owned()].to_vec()),
        );

        assert_eq!(
            de::<HashSet<u32>>(Value::Array([Value::U64(0), Value::U64(1)].to_vec())),
            HashSet::from([0, 1]),
        );
    }

    #[test]
    fn test_wrapper_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Unit;
        assert_eq!(de::<Unit>(Value::Null), Unit);

        #[derive(Deserialize, Debug, PartialEq)]
        struct Unit1();
        assert_eq!(de::<Unit1>(Value::Array(Vec::new())), Unit1());

        #[derive(Deserialize, Debug, PartialEq)]
        struct NewTypeStruct(i64);
        assert_eq!(de::<NewTypeStruct>(Value::I64(0)), NewTypeStruct(0));

        #[derive(Deserialize, Debug, PartialEq)]
        struct NewTypeStructTuple((i32,));
        assert_eq!(
            de::<NewTypeStructTuple>(Value::Array([Value::I64(0)].to_vec())),
            NewTypeStructTuple((0,))
        );

        #[derive(Deserialize, Debug, PartialEq)]
        struct TupleStruct(i32, String, (i32, i32), (), ((),));
        assert_eq!(
            de::<TupleStruct>(Value::Array(
                [
                    Value::I64(0),
                    Value::String("hello".to_owned()),
                    Value::Array([Value::I64(1), Value::I64(2)].to_vec()),
                    Value::Null,
                    Value::Array([Value::Null].to_vec()),
                ]
                .to_vec()
            )),
            TupleStruct(0, "hello".to_owned(), (1, 2), (), ((),))
        );
    }

    fn bool_true() -> bool {
        true
    }

    fn some_3() -> Option<u32> {
        Some(3)
    }

    #[test]
    fn test_map() {
        assert_eq!(
            de::<HashMap<i32, i32>>(Value::Map(Map::from([
                ("1".to_owned(), Value::I64(2)),
                ("3".to_owned(), Value::I64(4))
            ]))),
            HashMap::<i32, i32>::from([(1, 2), (3, 4)])
        );

        #[derive(Deserialize, Debug, PartialEq)]
        struct Struct {
            x: i32,
            #[serde(default = "bool_true")]
            b0: bool,
            #[serde(rename = "bb")]
            b1: bool,
            #[serde(flatten)]
            flat: Flat,
        }
        #[derive(Deserialize, Debug, PartialEq)]
        struct Flat {
            k: String,
            #[serde(default = "bool_true")]
            b1: bool,
            #[serde(default = "some_3")]
            opt: Option<u32>,
        }
        assert_eq!(
            de::<Struct>(Value::Map(Map::from([
                ("x".to_owned(), Value::I64(1)),
                ("bb".to_owned(), Value::Bool(false)),
                ("k".to_owned(), Value::String("hello".to_owned())),
            ]))),
            Struct {
                x: 1,
                b0: true,
                b1: false,
                flat: Flat {
                    k: "hello".to_owned(),
                    b1: true,
                    opt: Some(3),
                },
            }
        );
    }

    #[test]
    fn test_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum Enum {
            Var1,
            Var2,
            #[serde(rename = "hello")]
            Var3,
        }
        assert_eq!(de::<Enum>(Value::String("Var1".to_owned())), Enum::Var1);
        assert_eq!(de::<Enum>(Value::String("Var2".to_owned())), Enum::Var2);
        assert_eq!(de::<Enum>(Value::String("hello".to_owned())), Enum::Var3);

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(untagged)]
        enum Enum1 {
            A { a: u32 },
            BC { b: Option<u32>, c: Option<u32> },
        }
        assert_eq!(
            de::<Enum1>(Value::Map(Map::from([("a".to_owned(), Value::U64(0))]))),
            Enum1::A { a: 0 }
        );
        assert_eq!(
            de::<Enum1>(Value::Map(Map::new())),
            Enum1::BC { b: None, c: None }
        );

        #[derive(Deserialize, PartialEq, Debug)]
        enum Enum2 {
            A { a: u32 },
            BC { b: Option<u32>, c: Option<u32> },
            D,
            E(f32),
        }
        assert_eq!(
            de::<Enum2>(Value::Map(Map::from([(
                "A".to_owned(),
                Value::Map(Map::from([("a".to_owned(), Value::U64(0))]))
            )]))),
            Enum2::A { a: 0 }
        );
        assert_eq!(
            de::<Enum2>(Value::Map(Map::from([(
                "BC".to_owned(),
                Value::Map(Map::new())
            )]))),
            Enum2::BC { b: None, c: None }
        );
        assert_eq!(
            de::<Enum2>(Value::Map(Map::from([("D".to_owned(), Value::Null)]))),
            Enum2::D,
        );
        assert_eq!(
            de::<Enum2>(Value::Map(Map::from([("E".to_owned(), Value::F64(0.0),)]))),
            Enum2::E(0.0),
        );
    }

    #[test]
    fn test_decimal() {
        assert_eq!(de::<u32>(Value::Decimal(dec!(100.0))), 100);
        assert_eq!(de::<f32>(Value::Decimal(dec!(100))), 100.0);
        assert_eq!(de::<f64>(Value::Decimal(dec!(1999.1234))), 1999.1234);
        assert_eq!(
            de::<f64>(Value::Decimal(Decimal::MAX)),
            7.922816251426434e28
        );
        assert_eq!(de::<u64>(Value::Decimal(Decimal::from(u64::MAX))), u64::MAX);
    }
}
