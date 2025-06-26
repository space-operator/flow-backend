use std::borrow::Cow;

use crate::{Value, value_type::Variant};
use bincode::{
    Decode, Encode,
    config::standard,
    de::Decoder,
    enc::Encoder,
    error::{DecodeError, EncodeError},
};
use rust_decimal::Decimal;

pub struct MapBincode<'a>(pub Cow<'a, crate::Map>);

impl<'a> From<&'a crate::Map> for MapBincode<'a> {
    fn from(value: &'a crate::Map) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl<'a> From<crate::Map> for MapBincode<'a> {
    fn from(value: crate::Map) -> Self {
        Self(Cow::Owned(value))
    }
}

impl<'a, C> Decode<C> for MapBincode<'a> {
    fn decode<D: Decoder<Context = C>>(d: &mut D) -> Result<Self, DecodeError> {
        let len = decode_slice_len(d)?;
        d.claim_container_read::<(String, Value)>(len)?;

        let mut map = crate::Map::with_capacity(len);

        for _ in 0..len {
            d.unclaim_bytes_read(core::mem::size_of::<(String, Value)>());
            let key = String::decode(d)?;
            let value = Value::decode(d)?;
            map.insert(key, value);
        }

        Ok(Self(Cow::Owned(map)))
    }
}

impl<'a> Encode for MapBincode<'a> {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), EncodeError> {
        (self.0.len() as u64).encode(e)?;

        for (k, v) in self.0.iter() {
            k.encode(e)?;
            v.encode(e)?;
        }

        Ok(())
    }
}

pub fn map_to_bincode(map: &crate::Map) -> Result<Vec<u8>, EncodeError> {
    bincode::encode_to_vec(MapBincode::from(map), standard())
}

fn decode_slice_len<C, D: Decoder<Context = C>>(d: &mut D) -> Result<usize, DecodeError> {
    let v = u64::decode(d)?;

    v.try_into().map_err(|_| DecodeError::OutsideUsizeRange(v))
}

pub fn map_from_bincode(data: &[u8]) -> Result<crate::Map, DecodeError> {
    Ok(
        bincode::decode_from_slice::<MapBincode, _>(data, standard())?
            .0
            .0
            .into_owned(),
    )
}

impl Encode for Value {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), EncodeError> {
        (self.kind() as u8).encode(e)?;
        match self {
            crate::Value::Null => {}
            crate::Value::String(x) => x.encode(e)?,
            crate::Value::Bool(x) => x.encode(e)?,
            crate::Value::U64(x) => x.encode(e)?,
            crate::Value::I64(x) => x.encode(e)?,
            crate::Value::F64(x) => x.encode(e)?,
            crate::Value::Decimal(x) => x.serialize().encode(e)?,
            crate::Value::U128(x) => x.encode(e)?,
            crate::Value::I128(x) => x.encode(e)?,
            crate::Value::B32(x) => x.encode(e)?,
            crate::Value::B64(x) => x.encode(e)?,
            crate::Value::Bytes(x) => x.encode(e)?,
            crate::Value::Array(x) => x.encode(e)?,
            crate::Value::Map(x) => MapBincode::from(x).encode(e)?,
        }
        Ok(())
    }
}

impl<C> Decode<C> for crate::Value {
    fn decode<D: Decoder<Context = C>>(d: &mut D) -> Result<Self, DecodeError> {
        let kind_num = u8::decode(d)?;
        let kind =
            Variant::try_from(kind_num as u32).map_err(|_| DecodeError::UnexpectedVariant {
                type_name: "Value",
                allowed: &bincode::error::AllowedEnumVariants::Range {
                    min: Variant::MIN,
                    max: Variant::MAX,
                },
                found: kind_num as u32,
            })?;
        Ok(match kind {
            Variant::Null => Value::Null,
            Variant::String => Value::String(<_>::decode(d)?),
            Variant::Bool => Value::Bool(<_>::decode(d)?),
            Variant::U64 => Value::U64(<_>::decode(d)?),
            Variant::I64 => Value::I64(<_>::decode(d)?),
            Variant::F64 => Value::F64(<_>::decode(d)?),
            Variant::Decimal => Value::Decimal(Decimal::deserialize(<_>::decode(d)?)),
            Variant::I128 => Value::I128(<_>::decode(d)?),
            Variant::U128 => Value::U128(<_>::decode(d)?),
            Variant::B32 => Value::B32(<_>::decode(d)?),
            Variant::B64 => Value::B64(<_>::decode(d)?),
            Variant::Bytes => Value::Bytes(Vec::<u8>::decode(d)?.into()),
            Variant::Array => Value::Array(<_>::decode(d)?),
            Variant::Map => Value::Map(MapBincode::decode(d)?.0.into_owned()),
        })
    }
}

impl Value {
    pub fn to_bincode(&self) -> Result<Vec<u8>, EncodeError> {
        bincode::encode_to_vec(self, standard())
    }

    pub fn from_bincode(data: &[u8]) -> Result<Self, DecodeError> {
        bincode::decode_from_slice(data, standard()).map(|(value, _)| value)
    }
}
