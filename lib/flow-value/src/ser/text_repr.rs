use crate::{Five8Buffer32, Five8Buffer64, Value};
use base64::prelude::*;

#[derive(Debug)]
pub struct TextRepr<'a>(&'a Value);

impl TextRepr<'_> {
    pub fn new(value: &Value) -> TextRepr<'_> {
        TextRepr(value)
    }
}

impl serde::Serialize for TextRepr<'_> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        const NAME: &str = "TextRepr";

        let value = self.0;
        let (i, k) = value.kind().variant();
        match value {
            Value::Null => s.serialize_newtype_variant(NAME, i, k, &0),
            Value::String(v) => s.serialize_newtype_variant(NAME, i, k, v),
            Value::Bool(v) => s.serialize_newtype_variant(NAME, i, k, v),
            Value::U64(v) => {
                s.serialize_newtype_variant(NAME, i, k, itoa::Buffer::new().format(*v))
            }
            Value::I64(v) => {
                s.serialize_newtype_variant(NAME, i, k, itoa::Buffer::new().format(*v))
            }
            Value::F64(v) => s.serialize_newtype_variant(NAME, i, k, ryu::Buffer::new().format(*v)),
            Value::Decimal(v) => {
                // TODO: no alloc impl
                s.serialize_newtype_variant(NAME, i, k, &v.to_string())
            }
            Value::I128(v) => {
                s.serialize_newtype_variant(NAME, i, k, itoa::Buffer::new().format(*v))
            }
            Value::U128(v) => {
                s.serialize_newtype_variant(NAME, i, k, itoa::Buffer::new().format(*v))
            }
            Value::B32(v) => {
                s.serialize_newtype_variant(NAME, i, k, Five8Buffer32::new().encode(v))
            }
            Value::B64(v) => {
                s.serialize_newtype_variant(NAME, i, k, Five8Buffer64::new().encode(v))
            }
            Value::Bytes(v) => s.serialize_newtype_variant(NAME, i, k, &BASE64_STANDARD.encode(v)),
            Value::Array(v) => s.serialize_newtype_variant(
                NAME,
                i,
                k,
                &super::iter_ser::Array::new(v.iter().map(Self::new)),
            ),
            Value::Map(v) => s.serialize_newtype_variant(
                NAME,
                i,
                k,
                &super::iter_ser::Map::new(v.iter().map(|(k, v)| (k, Self::new(v)))),
            ),
        }
    }
}
