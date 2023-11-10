use crate::Value;

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Variant {
    Null = 0,
    String = 1,
    Bool = 2,
    U64 = 3,
    I64 = 4,
    F64 = 5,
    Decimal = 6,
    I128 = 7,
    U128 = 8,
    B32 = 9,
    B64 = 10,
    Bytes = 11,
    Array = 12,
    Map = 13,
}

impl Variant {
    pub const fn variant(&self) -> (u32, &'static str) {
        let idx = *self as u32;
        (idx, keys::ALL[idx as usize])
    }
}

pub mod keys {
    pub const NULL: &str = "N";
    pub const STRING: &str = "S";
    pub const BOOL: &str = "B";
    pub const U64: &str = "U";
    pub const I64: &str = "I";
    pub const F64: &str = "F";
    pub const DECIMAL: &str = "D";
    pub const I128: &str = "I1";
    pub const U128: &str = "U1";
    pub const B32: &str = "B3";
    pub const B64: &str = "B6";
    pub const BYTES: &str = "BY";
    pub const ARRAY: &str = "A";
    pub const MAP: &str = "M";

    pub const ALL: &[&str] = &[
        NULL, STRING, BOOL, U64, I64, F64, DECIMAL, I128, U128, B32, B64, BYTES, ARRAY, MAP,
    ];
}

struct ValueTypeVisitor;

impl<'de> serde::de::Visitor<'de> for ValueTypeVisitor {
    type Value = Variant;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("ValueType")
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(match v {
            0 => Variant::Null,
            1 => Variant::String,
            2 => Variant::Bool,
            3 => Variant::U64,
            4 => Variant::I64,
            5 => Variant::F64,
            6 => Variant::Decimal,
            7 => Variant::I128,
            8 => Variant::U128,
            9 => Variant::B32,
            10 => Variant::B64,
            11 => Variant::Bytes,
            12 => Variant::Array,
            13 => Variant::Map,
            _ => {
                return Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(v as u64),
                    &"value in [0, 13]",
                ))
            }
        })
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(match v {
            keys::NULL => Variant::Null,
            keys::STRING => Variant::String,
            keys::BOOL => Variant::Bool,
            keys::U64 => Variant::U64,
            keys::I64 => Variant::I64,
            keys::F64 => Variant::F64,
            keys::DECIMAL => Variant::Decimal,
            keys::I128 => Variant::I128,
            keys::U128 => Variant::U128,
            keys::B32 => Variant::B32,
            keys::B64 => Variant::B64,
            keys::BYTES => Variant::Bytes,
            keys::ARRAY => Variant::Array,
            keys::MAP => Variant::Map,
            _ => {
                return Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &"one of valid keys",
                ))
            }
        })
    }
}

impl<'de> serde::Deserialize<'de> for Variant {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if d.is_human_readable() {
            d.deserialize_str(ValueTypeVisitor)
        } else {
            d.deserialize_u32(ValueTypeVisitor)
        }
    }
}

impl Value {
    pub fn kind(&self) -> Variant {
        match self {
            Value::Null => Variant::Null,
            Value::String(_) => Variant::String,
            Value::Bool(_) => Variant::Bool,
            Value::U64(_) => Variant::U64,
            Value::I64(_) => Variant::I64,
            Value::F64(_) => Variant::F64,
            Value::Decimal(_) => Variant::Decimal,
            Value::I128(_) => Variant::I128,
            Value::U128(_) => Variant::U128,
            Value::B32(_) => Variant::B32,
            Value::B64(_) => Variant::B64,
            Value::Bytes(_) => Variant::Bytes,
            Value::Array(_) => Variant::Array,
            Value::Map(_) => Variant::Map,
        }
    }
}
