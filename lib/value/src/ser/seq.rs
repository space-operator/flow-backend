use serde::ser::Impossible;

use super::Serializer;
use crate::{Error, Value};

pub enum SerializeSeq {
    Bytes(Vec<u8>),
    Array(Vec<Value>),
}

impl Default for SerializeSeq {
    fn default() -> Self {
        Self::new()
    }
}

impl SerializeSeq {
    pub fn new() -> Self {
        SerializeSeq::Bytes(Vec::new())
    }
}

impl TryFrom<Value> for u8 {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::U64(x) => u8::try_from(x).map_err(|_| value),
            Value::I64(x) => u8::try_from(x).map_err(|_| value),
            Value::U128(x) => u8::try_from(x).map_err(|_| value),
            value => Err(value),
        }
    }
}

impl serde::ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        match self {
            Self::Array(vec) => {
                let value = value.serialize(Serializer)?;
                vec.push(value);
            }
            Self::Bytes(vec) => {
                let value = value.serialize(Serializer)?;
                match u8::try_from(value) {
                    Ok(v) => vec.push(v),
                    Err(v) => {
                        let Self::Bytes(old) = std::mem::replace(self, Self::Array(Vec::new()))
                        else {
                            unreachable!()
                        };
                        let Self::Array(new) = self else {
                            unreachable!()
                        };
                        new.extend(old.into_iter().map(Value::from).chain(std::iter::once(v)));
                    }
                }
            }
        }
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(match self {
            Self::Bytes(vec) => {
                if vec.is_empty() {
                    Value::Array(Vec::new())
                } else {
                    Value::from(vec)
                }
            }
            Self::Array(vec) => Value::Array(vec),
        })
    }
}

impl serde::ser::SerializeTuple for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleStruct for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[derive(Default)]
pub struct SerializeSeqNoBytes {
    array: Vec<Value>,
}

impl serde::ser::SerializeSeq for SerializeSeqNoBytes {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let value = value.serialize(Serializer)?;
        self.array.push(value);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Array(self.array))
    }
}

impl serde::Serializer for SerializeSeqNoBytes {
    type Ok = Value;

    type Error = Error;

    type SerializeSeq = Self;

    type SerializeTuple = Impossible<Value, Error>;

    type SerializeTupleStruct = Impossible<Value, Error>;

    type SerializeTupleVariant = Impossible<Value, Error>;

    type SerializeMap = Impossible<Value, Error>;

    type SerializeStruct = Impossible<Value, Error>;

    type SerializeStructVariant = Impossible<Value, Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Err(Error::ExpectedArray)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Err(Error::ExpectedArray)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Err(Error::ExpectedArray)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(Self {
            array: len.map(Vec::with_capacity).unwrap_or_default(),
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Error::ExpectedArray)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::ExpectedArray)
    }
}
