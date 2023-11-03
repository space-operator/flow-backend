use crate::{Error, Map, Value};

mod iter_ser;
mod map_key;
mod maps;
mod seq;
mod tagged_bytes;
mod text_repr;

use maps::{SerializeMap, SerializeStructVariant, SerializeTupleVariant};
use seq::{SerializeSeq, SerializeSeqNoBytes};
use tagged_bytes::TaggedBytes;

impl serde::Serialize for Value {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::TOKEN;

        let (i, k) = self.kind().variant();
        if s.is_human_readable() {
            text_repr::TextRepr::new(self).serialize(s)
        } else {
            match self {
                Value::Null => s.serialize_newtype_variant(TOKEN, i, k, &()),
                Value::String(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::Bool(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::U64(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::I64(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::F64(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::Decimal(v) => {
                    s.serialize_newtype_variant(TOKEN, i, k, &crate::Bytes(&v.serialize()))
                }
                Value::I128(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::U128(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::B32(v) => s.serialize_newtype_variant(TOKEN, i, k, &crate::Bytes(v)),
                Value::B64(v) => s.serialize_newtype_variant(TOKEN, i, k, &crate::Bytes(v)),
                Value::Bytes(v) => s.serialize_newtype_variant(TOKEN, i, k, &crate::Bytes(v)),
                Value::Array(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
                Value::Map(v) => s.serialize_newtype_variant(TOKEN, i, k, &v),
            }
        }
    }
}

/// Turn any type that implements `Serialize` into `Value`.
pub struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeSeq;
    type SerializeTupleStruct = SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(String::from(v)))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        v.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        match name {
            crate::decimal::TOKEN => v.serialize(TaggedBytes::Decimal),
            #[cfg(feature = "solana")]
            crate::keypair::TOKEN | crate::signature::TOKEN | crate::pubkey::TOKEN => {
                v.serialize(TaggedBytes::Bytes)
            }
            _ => v.serialize(self),
        }
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq::new())
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap {
            map: Map::with_capacity(len.unwrap_or(0)),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeMap {
            map: Map::with_capacity(len),
            next_key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant {
            name: variant,
            map: Map::with_capacity(len),
        })
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        if name == crate::TOKEN {
            match index {
                // Decimal's index
                6 => value.serialize(TaggedBytes::Decimal),
                // Other bytes
                9..=11 => value.serialize(TaggedBytes::Bytes),
                // Array
                12 => value.serialize(SerializeSeqNoBytes::default()),
                // Other variants can map directly to serde's data model
                _ => value.serialize(Serializer),
            }
        } else {
            let value = value.serialize(Serializer)?;
            Ok(Value::Map(Map::from([(variant.to_owned(), value)])))
        }
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant {
            name: variant,
            seq: SerializeSeq::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde::Serialize;
    use std::collections::HashMap;

    #[test]
    fn test_value_to_value() {
        fn t(v: Value) {
            assert_eq!(v.serialize(Serializer).unwrap(), v);
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

    fn s<T: Serialize>(t: T) -> Value {
        t.serialize(Serializer).unwrap()
    }

    #[test]
    fn test_serialize_primitive() {
        assert_eq!(s(0u8), Value::U64(0));
        assert_eq!(s(0u16), Value::U64(0));
        assert_eq!(s(0u32), Value::U64(0));
        assert_eq!(s(0u64), Value::U64(0));
        assert_eq!(s(0i8), Value::I64(0));
        assert_eq!(s(0i16), Value::I64(0));
        assert_eq!(s(0i32), Value::I64(0));
        assert_eq!(s(0i64), Value::I64(0));
        assert_eq!(s(0f32), Value::F64(0.0));
        assert_eq!(s(0f64), Value::F64(0.0));
        assert_eq!(s(true), Value::Bool(true));
        assert_eq!(s(Option::<()>::None), Value::Null);
        assert_eq!(s(()), Value::Null);
        assert_eq!(s("end"), Value::String("end".to_owned()));
        assert_eq!(
            s((1i32, -2i32, "hello")),
            Value::Array(vec![
                // SerializerSeq does not retain the original type
                Value::U64(1),
                Value::I64(-2),
                Value::String("hello".to_owned())
            ])
        );
        assert_eq!(s([0u8; 0]), Value::Array(Vec::new()));
        assert_eq!(s([()]), Value::Array([Value::Null].to_vec()));
        assert_eq!(
            s(HashMap::from([("a".to_owned(), -1i32)])),
            Value::Map(Map::from([("a".to_owned(), Value::I64(-1))]))
        );
        assert_eq!(s([1u8; 32]), Value::B32([1; 32]));
        assert_eq!(s(crate::Bytes(&[2u8; 64])), Value::B64([2; 64]));
    }

    #[test]
    fn test_derive() {
        #[derive(Serialize)]
        struct A {
            a: Noop,
            b: B,
            c0: C,
            c1: C,
            #[serde(flatten)]
            f: C,
            #[serde(rename = "NULL")]
            null: Option<i32>,
            i: I32,
        }

        #[derive(Serialize)]
        struct Noop;

        #[derive(Serialize)]
        struct B {}

        #[derive(Serialize)]
        struct I32(i32);

        #[derive(Serialize)]
        struct C {
            #[serde(skip_serializing_if = "Option::is_none")]
            y: Option<i32>,
        }

        assert_eq!(
            s(A {
                a: Noop,
                b: B {},
                c0: C { y: None },
                c1: C { y: Some(1) },
                f: C { y: Some(2) },
                null: None,
                i: I32(323232),
            }),
            Value::Map(
                [
                    ("a".into(), Value::Null),
                    ("b".into(), Value::Map(Map::new())),
                    ("c0".into(), Value::Map(Map::new())),
                    (
                        "c1".into(),
                        Value::Map([("y".into(), Value::I64(1))].into())
                    ),
                    ("y".into(), Value::I64(2)),
                    ("NULL".into(), Value::Null),
                    ("i".into(), Value::I64(323232)),
                ]
                .into()
            )
        );
    }

    #[test]
    fn test_enum() {
        #[derive(Serialize, Debug, PartialEq)]
        enum Enum0 {
            V0,
            V1,
            #[serde(rename = "var")]
            V2,
            V3(i32),
        }
        assert_eq!(s(Enum0::V0), Value::String("V0".to_owned()));
        assert_eq!(s(Enum0::V1), Value::String("V1".to_owned()));
        assert_eq!(s(Enum0::V2), Value::String("var".to_owned()));
        assert_eq!(
            s(Enum0::V3(-1)),
            Value::Map(Map::from([("V3".to_owned(), Value::I64(-1))]))
        );
    }
}
