use crate::{Error, Value};
use rust_decimal::Decimal;
use serde::ser::Impossible;

pub enum TaggedBytes {
    Decimal,
    Bytes,
}

impl serde::Serializer for TaggedBytes {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        match self {
            Self::Decimal => Ok(Value::Decimal(Decimal::deserialize(
                value.try_into().map_err(|_| Error::InvalidLenght {
                    need: 16,
                    got: value.len(),
                })?,
            ))),
            Self::Bytes => Ok(Value::from(value)),
        }
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_newtype_struct<T>(self, _: &'static str, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        unreachable!();
    }

    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        unreachable!();
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        unreachable!();
    }

    fn serialize_some<T>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        unreachable!();
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        unreachable!();
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unreachable!();
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unreachable!();
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unreachable!();
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unreachable!();
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        unreachable!();
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unreachable!();
    }
}
