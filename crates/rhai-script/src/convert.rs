use flow_lib::{Name, Value, value::Decimal};
use rhai::Dynamic;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum ConvertError {
    #[error("unknown type: {}", .0)]
    UnknownType(&'static str),
}

pub fn dynamic_to_value(v: Dynamic) -> Result<Value, ConvertError> {
    if v.is::<Value>() {
        Ok(v.cast::<Value>())
    } else if v.is_string() {
        v.into_string()
            .map(Value::String)
            .map_err(ConvertError::UnknownType)
    } else if v.is_map() {
        let map = v.cast::<rhai::Map>();
        Ok(Value::Map(
            map.into_iter()
                .map(|(k, v)| Ok((Name::from(k), dynamic_to_value(v)?)))
                .collect::<Result<flow_lib::value::Map, _>>()?,
        ))
    } else if v.is_array() {
        let array = v.cast::<rhai::Array>();
        Ok(Value::Array(
            array
                .into_iter()
                .map(dynamic_to_value)
                .collect::<Result<Vec<Value>, _>>()?,
        ))
    } else if v.as_unit().is_ok() {
        Ok(Value::Null)
    } else if let Ok(d) = v.as_decimal() {
        Ok(Value::Decimal(d))
    } else if let Ok(i) = v.as_int() {
        Ok(Value::I64(i))
    } else if let Ok(f) = v.as_float() {
        Ok(Value::F64(f))
    } else if let Ok(b) = v.as_bool() {
        Ok(Value::Bool(b))
    } else {
        Err(ConvertError::UnknownType(v.type_name()))
    }
}

pub fn value_to_dynamic(v: Value) -> Dynamic {
    match v {
        Value::Null => Dynamic::UNIT,
        Value::String(x) => x.into(),
        Value::Bool(x) => x.into(),
        Value::U64(x) => Decimal::from(x).into(),
        Value::I64(x) => x.into(),
        Value::F64(x) => x.into(),
        Value::Decimal(x) => x.into(),
        Value::U128(x) => Dynamic::from(x), // TODO
        Value::I128(x) => Dynamic::from(x), // TODO
        Value::B32(x) => bs58::encode(&x).into_string().into(),
        Value::B64(x) => bs58::encode(&x).into_string().into(),
        Value::Bytes(x) => rhai::Blob::from(x).into(),
        Value::Array(x) => x
            .into_iter()
            .map(value_to_dynamic)
            .collect::<rhai::Array>()
            .into(),
        Value::Map(x) => x
            .into_iter()
            .map(|(k, v)| (k.into(), value_to_dynamic(v)))
            .collect::<rhai::Map>()
            .into(),
    }
}
