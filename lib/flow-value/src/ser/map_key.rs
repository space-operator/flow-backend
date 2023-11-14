use crate::Error;
use serde::ser::Impossible;

pub(crate) struct MapKeySerializer;

const fn key_must_be_a_string() -> Error {
    Error::KeyMustBeAString
}

impl serde::Serializer for MapKeySerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<String, Error> {
        Ok(variant.to_owned())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<String, Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_bool(self, _value: bool) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_i8(self, value: i8) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_i16(self, value: i16) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_i32(self, value: i32) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_i64(self, value: i64) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_u8(self, value: u8) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_u16(self, value: u16) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_u32(self, value: u32) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_u64(self, value: u64) -> Result<String, Error> {
        Ok(itoa::Buffer::new().format(value).to_owned())
    }

    fn serialize_f32(self, _value: f32) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_f64(self, _value: f64) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_char(self, value: char) -> Result<String, Error> {
        Ok(String::from(value))
    }

    fn serialize_str(self, value: &str) -> Result<String, Error> {
        Ok(value.to_owned())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit(self) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String, Error>
    where
        T: ?Sized + serde::Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_none(self) -> Result<String, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_some<T>(self, _value: &T) -> Result<String, Error>
    where
        T: ?Sized + serde::Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(key_must_be_a_string())
    }

    fn collect_str<T>(self, value: &T) -> Result<String, Error>
    where
        T: ?Sized + std::fmt::Display,
    {
        Ok(value.to_string())
    }
}
