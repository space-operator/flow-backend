//! This crate contains [`Value`], an enum representing all values that can be used as
//! node's input and output, and utilities for working with [`Value`].
//!
//! Common operations:
//! - Converting [`Value`] to Rust types.
//! - Converting Rust types to [`Value`].
//! - Receiving [`value::Map`][Map] as node's input.
//! - Returning [`value::Map`][Map] as node's output.
//! - Converting [`Value`] to/from JSON to use in HTTP APIs and database.
//! - Getting and updating nested values with JSON Pointer syntax.

use rust_decimal::prelude::ToPrimitive;
use thiserror::Error as ThisError;

pub use rust_decimal::Decimal;

pub(crate) mod value_type;
pub use value_type::keys;

pub(crate) const TOKEN: &str = "$V";

mod de;
pub use de::const_bytes::ConstBytes;

mod ser;

pub mod crud;
pub mod macros;

// custom serialize and deserialize modules
pub mod decimal;
#[cfg(feature = "solana")]
pub mod keypair;
#[cfg(feature = "solana")]
pub mod pubkey;
#[cfg(feature = "solana")]
pub mod signature;

/// Interpret a [`Value`] as an instance of type `T`
///
/// # Example
///
/// ```
/// use solana_sdk::pubkey::Pubkey;
/// use solana_sdk::pubkey;
/// use value::Value;
///
/// #[derive(serde::Deserialize)]
/// pub struct User {
///     pubkey: Pubkey,
/// }
///
/// let value = Value::Map(value::map! {
///     "pubkey" => pubkey!("My11111111111111111111111111111111111111111"),
/// });
/// value::from_value::<User>(value).unwrap();
/// ```
pub fn from_value<T>(value: Value) -> Result<T, Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    T::deserialize(value)
}

/// Interpret a [`Map`] as an instance of type `T`
///
/// # Example
///
/// ```
/// use solana_sdk::pubkey::Pubkey;
/// use solana_sdk::pubkey;
/// use value::Value;
///
/// #[derive(serde::Deserialize)]
/// pub struct User {
///     pubkey: Pubkey,
/// }
///
/// let map = value::map! {
///     "pubkey" => pubkey!("My11111111111111111111111111111111111111111"),
/// };
/// value::from_map::<User>(map).unwrap();
/// ```
pub fn from_map<T>(map: Map) -> Result<T, Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    T::deserialize(Value::Map(map))
}

/// Convert a `T` into [`Value`].
///
/// # Example
///
/// ```
/// use solana_sdk::signature::Signature;
/// use value::Value;
///
/// let val = value::to_value(&Signature::new_unique()).unwrap();
/// assert!(matches!(val, Value::B64(_)));
/// ```
pub fn to_value<T>(t: &T) -> Result<Value, Error>
where
    T: serde::Serialize,
{
    t.serialize(ser::Serializer)
}

/// Convert a `T` into [`Map`].
///
/// # Example
///
/// ```
/// use value::Value;
///
/// let map = value::to_map(&serde_json::json!({"A": "B"})).unwrap();
/// assert_eq!(map, value::map! { "A" => "B" });
/// ```
pub fn to_map<T>(t: &T) -> Result<Map, Error>
where
    T: serde::Serialize,
{
    to_value(t).and_then(|v| {
        if let Value::Map(map) = v {
            Ok(map)
        } else {
            Err(Error::ExpectedMap)
        }
    })
}

/// Allow for switching HashMap implementation
pub type HashMap<K, V> = indexmap::IndexMap<K, V>;

/// Key type of [`Map`]
pub type Key = String;

pub type Map = self::HashMap<Key, Value>;

/// [`Value`] represents all values that nodes can use as input and output.
///
/// # Data Types
///
/// - Scalar types:
///     - Null: [`Value::Null`].
///     - Boolean: [`Value::Bool`].
///     - Numbers: [`Value::U64`], [`Value::I64`], [`Value::U128`], [`Value::I128`], [`Value::Decimal`],
///     [`Value::F64`].
///     - String: [`Value::String`].
///     - Binary: [`Value::B32`], [`Value::B64`], [`Value::Bytes`].
/// - Array: [`Value::Array`]
/// - Map: [`Value::Map`]
///
/// # Node Input
///
/// Node receives a [`value::Map`][Map] as its input. It is possible to use the map directly, but
/// it is often preferred to convert it to structs or enums of your choice.
///
/// [`Value`] implements [`Deserializer`][serde::Deserializer], therefore it can be converted to
/// any types supported by Serde. We provide 2 helpers:
///
/// - [`value::from_value`][from_value] - [`Value`] to any `T: Deserialize`.
/// - [`value::from_map`][from_map] - [`Map`] to any `T: Deserialize`.
///
/// # Node Output
///
/// Node returns a [`value::Map`][Map] as its output.
///
/// Building the output directly with [`value::map!`][macro@map] and
/// [`value::array!`][macro@array] macros:
/// ```
/// let value = value::map! {
///     "customer_name" => "John",
///     "items" => value::array![1, 2, 3],
/// };
/// ```
///
/// [`Value`] also implements [`Serializer`][serde::Serializer], you can use
/// [`value::to_map`][to_map] to convert any type `T: Serialize` into [`value::Map`][Map].
///
/// ```
/// #[derive(serde::Serialize)]
/// struct Order {
///     customer_name: String,
///     items: Vec<i32>,
/// }
///
/// value::to_map(&Order {
///     customer_name: "John".to_owned(),
///     items: [1, 2, 3].into(),
/// })
/// .unwrap();
/// ```
///
/// # JSON representation
///
/// When using [`Value`] in database and HTTP APIs, it is converted to a JSON object:
///
/// ```json
/// {
///     "<variant identifier>": <data>
/// }
/// ```
///
/// Identifiers of each enum variant:
/// - **N**: [`Value::Null`]
/// - **S**: [`Value::String`]
/// - **B**: [`Value::Bool`]
/// - **U**: [`Value::U64`]
/// - **I**: [`Value::I64`]
/// - **F**: [`Value::F64`]
/// - **D**: [`Value::Decimal`]
/// - **U1**: [`Value::U128`]
/// - **I1**: [`Value::I128`]
/// - **B3**: [`Value::B32`]
/// - **B6**: [`Value::B64`]
/// - **BY**: [`Value::Bytes`]
/// - **A**: [`Value::Array`]
/// - **M**: [`Value::Map`]
///
/// See variant's documentation to see how data are encoded.
///
/// Use [`serde_json`] to encode and decode [`Value`] as JSON:
/// ```
/// use value::Value;
///
/// let value = Value::U64(10);
///
/// // encode Value to JSON
/// let json = serde_json::to_string(&value).unwrap();
/// assert_eq!(json, r#"{"U":"10"}"#);
///
/// // decode JSON to Value
/// let value1 = serde_json::from_str::<Value>(&json).unwrap();
/// assert_eq!(value1, value);
/// ```
#[derive(Clone, PartialEq, Default)]
pub enum Value {
    /// JSON representation:
    /// ```json
    /// { "N": 0 }
    /// ```
    #[default]
    Null,
    /// UTF-8 string.
    ///
    /// JSON representation:
    /// ```json
    /// { "S": "hello" }
    /// ```
    String(String),
    /// JSON representation:
    /// ```json
    /// { "B": true }
    /// ```
    Bool(bool),
    /// JSON representation:
    /// ```json
    /// { "U": "100" }
    /// ```
    ///
    /// Numbers are encoded as JSON string to avoid losing precision when reading them in
    /// Javascript/Typescript.
    U64(u64),
    /// JSON representation:
    /// ```json
    /// { "I": "-100" }
    /// ```
    I64(i64),
    /// JSON representation:
    /// ```json
    /// { "F": "0.0" }
    /// ```
    /// Scientific notation is supported:
    /// ```json
    /// { "F": "1e9" }
    /// ```
    F64(f64),
    /// [`rust_decimal::Decimal`], suitable for financial calculations.
    ///
    /// JSON representation:
    /// ```json
    /// { "D": "3.1415926535897932384626433832" }
    /// ```
    Decimal(Decimal),
    /// JSON representation:
    /// ```json
    /// { "U1": "340282366920938463463374607431768211455" }
    /// ```
    U128(u128),
    /// JSON representation:
    /// ```json
    /// { "I1": "-170141183460469231731687303715884105728" }
    /// ```
    I128(i128),
    /// 32-bytes binary values, usually a Solana public key.
    ///
    /// JSON representation: encoded as a base-58 string
    /// ```json
    /// { "B3": "FMQUifdAHTytSxhiK4N7LmpvKRZaUmBnNnZmzFsdTPHB" }
    /// ```
    B32([u8; 32]),
    /// 64-bytes binary values, usually a Solana signature or keypair.
    ///
    /// JSON representation: encoded as a base-58 string
    /// ```json
    /// { "B6": "4onDpbfeT7nNN9MNMvTEZRn6pbtrQc1pdTBJB4a7HbfhAE6c5bkbuuFfYtkqs99hAqp5o6j7W1VyuKDxCn79k3Tk" }
    /// ```
    B64([u8; 64]),
    /// Binary values with length other than 32 and 64.
    ///
    /// JSON representation: encoded as a base-64 string
    /// ```json
    /// { "BY": "UmFpbnk=" }
    /// ```
    Bytes(bytes::Bytes),
    /// An array of [`Value`]. Array can contains other arrays, maps, ect. Array elements do not
    /// have to be of the same type.
    ///
    /// JSON representation:
    ///
    /// Example array containing a number and a string:
    /// ```json
    /// {
    ///     "A": [
    ///         { "U": 0 },
    ///         { "S": "hello" }
    ///     ]
    /// }
    /// ```
    Array(Vec<Self>),
    /// A key-value map, implemented with [`indexmap::IndexMap`], will preserve insertion order.
    /// Keys are strings and values can be any [`Value`].
    ///
    /// JSON representation:
    /// ```json
    /// {
    ///     "M": {
    ///         "first name": { "S": "John" },
    ///         "age": { "U": "20" }
    ///     }
    /// }
    /// ```
    Map(Map),
}

impl Value {
    pub fn new_keypair_bs58(s: &str) -> Result<Self, Error> {
        // and Ed25519 keypair
        const KEYPAIR_LENGTH: usize = 64;
        let mut buf = [0u8; KEYPAIR_LENGTH];
        let size = bs58::decode(s).into(&mut buf)?;
        if size != KEYPAIR_LENGTH {
            return Err(Error::InvalidLenght {
                need: KEYPAIR_LENGTH,
                got: size,
            });
        }

        Ok(Value::B64(buf))
    }

    pub fn normalize(self) -> Self {
        match self {
            Value::Null
            | Value::String(_)
            | Value::Bool(_)
            | Value::U64(_)
            | Value::I64(_)
            | Value::F64(_)
            | Value::B32(_)
            | Value::B64(_)
            | Value::Bytes(_) => self,
            Value::Decimal(mut d) => {
                d.normalize_assign();
                if d.scale() == 0 {
                    Value::I128(d.to_i128().expect("always fit into i128")).normalize()
                } else {
                    Value::Decimal(d)
                }
            }
            Value::I128(i) => if i < 0 {
                i64::try_from(i).map(Value::I64).ok()
            } else {
                u64::try_from(i).map(Value::U64).ok()
            }
            .unwrap_or(self),
            Value::U128(u) => u64::try_from(u).map(Value::U64).unwrap_or(self),
            Value::Array(mut a) => {
                for v in &mut a {
                    *v = std::mem::take(v).normalize();
                }
                Value::Array(a)
            }
            Value::Map(mut m) => {
                for v in m.values_mut() {
                    *v = std::mem::take(v).normalize();
                }
                Value::Map(m)
            }
        }
    }
}

#[cfg(feature = "json")]
mod json {
    use crate::Value;
    use rust_decimal::Decimal;

    impl From<serde_json::Value> for Value {
        fn from(value: serde_json::Value) -> Self {
            match value {
                serde_json::Value::Null => Value::Null,
                serde_json::Value::Bool(b) => Value::Bool(b),
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        Value::U64(u)
                    } else if let Some(i) = n.as_i64() {
                        if i < 0 {
                            Value::I64(i)
                        } else {
                            Value::U64(i as u64)
                        }
                    } else {
                        let s = n.to_string();
                        if let Ok(u) = s.parse::<u128>() {
                            Value::U128(u)
                        } else if let Ok(i) = s.parse::<i128>() {
                            Value::I128(i)
                        } else if let Ok(d) = s.parse::<Decimal>() {
                            Value::Decimal(d)
                        } else if let Ok(d) = Decimal::from_scientific(&s) {
                            Value::Decimal(d)
                        } else if let Ok(f) = s.parse::<f64>() {
                            Value::F64(f)
                        } else {
                            // unlikely to happen
                            // if happen, probably a bug in serde_json
                            Value::String(s)
                        }
                    }
                }
                serde_json::Value::String(s) => Value::String(s),
                serde_json::Value::Array(vec) => {
                    Value::Array(vec.into_iter().map(Value::from).collect())
                }
                serde_json::Value::Object(map) => {
                    Value::Map(map.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
                }
            }
        }
    }

    impl From<Value> for serde_json::Value {
        fn from(value: Value) -> Self {
            match value {
                Value::Null => serde_json::Value::Null,
                Value::String(x) => x.into(),
                Value::Bool(x) => x.into(),
                Value::U64(x) => x.into(),
                Value::I64(x) => x.into(),
                Value::F64(x) => x.into(),
                Value::Array(x) => x.into(),
                Value::Map(x) => x
                    .into_iter()
                    .map(|(key, value)| (key, value.into()))
                    .collect::<serde_json::Map<_, _>>()
                    .into(),
                Value::U128(value) => value
                    .try_into()
                    .map(u64::into)
                    .unwrap_or_else(|_| (value as f64).into()),
                Value::I128(value) => value
                    .try_into()
                    .map(i64::into)
                    .unwrap_or_else(|_| (value as f64).into()),
                Value::Decimal(mut d) => {
                    d.normalize_assign();
                    if d.scale() == 0 {
                        if let Ok(n) = u64::try_from(d) {
                            n.into()
                        } else if let Ok(n) = i64::try_from(d) {
                            n.into()
                        } else {
                            f64::try_from(d).map_or(serde_json::Value::Null, Into::into)
                        }
                    } else {
                        f64::try_from(d).map_or(serde_json::Value::Null, Into::into)
                    }
                }
                Value::B32(b) => (&b[..]).into(),
                Value::B64(b) => (&b[..]).into(),
                Value::Bytes(b) => (&b[..]).into(),
            }
        }
    }
}

impl From<String> for Value {
    fn from(x: String) -> Self {
        Self::String(x)
    }
}

impl From<&str> for Value {
    fn from(x: &str) -> Self {
        Self::String(x.to_owned())
    }
}

impl From<bool> for Value {
    fn from(x: bool) -> Self {
        Self::Bool(x)
    }
}

impl From<u8> for Value {
    fn from(x: u8) -> Self {
        Self::U64(x as u64)
    }
}

impl From<u16> for Value {
    fn from(x: u16) -> Self {
        Self::U64(x as u64)
    }
}

impl From<u32> for Value {
    fn from(x: u32) -> Self {
        Self::U64(x as u64)
    }
}

impl From<u64> for Value {
    fn from(x: u64) -> Self {
        Self::U64(x)
    }
}

impl From<u128> for Value {
    fn from(x: u128) -> Self {
        Self::U128(x)
    }
}

impl From<i8> for Value {
    fn from(x: i8) -> Self {
        Self::I64(x as i64)
    }
}

impl From<i16> for Value {
    fn from(x: i16) -> Self {
        Self::I64(x as i64)
    }
}

impl From<i32> for Value {
    fn from(x: i32) -> Self {
        Self::I64(x as i64)
    }
}

impl From<i64> for Value {
    fn from(x: i64) -> Self {
        Self::I64(x)
    }
}

impl From<i128> for Value {
    fn from(x: i128) -> Self {
        Self::I128(x)
    }
}

impl From<Decimal> for Value {
    fn from(x: Decimal) -> Self {
        Self::Decimal(x)
    }
}

impl From<f32> for Value {
    fn from(x: f32) -> Self {
        Self::F64(x as f64)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Self {
        Self::F64(x)
    }
}

impl From<[u8; 32]> for Value {
    fn from(x: [u8; 32]) -> Self {
        Self::B32(x)
    }
}

impl From<[u8; 64]> for Value {
    fn from(x: [u8; 64]) -> Self {
        Self::B64(x)
    }
}

#[cfg(feature = "solana")]
impl From<solana_sdk::pubkey::Pubkey> for Value {
    fn from(x: solana_sdk::pubkey::Pubkey) -> Self {
        Self::B32(x.to_bytes())
    }
}

#[cfg(feature = "solana")]
impl From<solana_sdk::signer::keypair::Keypair> for Value {
    fn from(x: solana_sdk::signer::keypair::Keypair) -> Self {
        Self::B64(x.to_bytes())
    }
}

#[cfg(feature = "solana")]
impl From<solana_sdk::signature::Signature> for Value {
    fn from(x: solana_sdk::signature::Signature) -> Self {
        Self::B64(x.into())
    }
}

impl From<bytes::Bytes> for Value {
    fn from(x: bytes::Bytes) -> Self {
        match x.len() {
            32 => Self::B32(<_>::try_from(&*x).unwrap()),
            64 => Self::B64(<_>::try_from(&*x).unwrap()),
            _ => Self::Bytes(x),
        }
    }
}

impl From<&[u8]> for Value {
    fn from(x: &[u8]) -> Self {
        match x.len() {
            32 => Self::B32(<_>::try_from(x).unwrap()),
            64 => Self::B64(<_>::try_from(x).unwrap()),
            _ => Self::Bytes(bytes::Bytes::copy_from_slice(x)),
        }
    }
}

impl From<Vec<u8>> for Value {
    fn from(x: Vec<u8>) -> Self {
        match x.len() {
            32 => Self::B32(<_>::try_from(&*x).unwrap()),
            64 => Self::B64(<_>::try_from(&*x).unwrap()),
            _ => Self::Bytes(x.into()),
        }
    }
}

impl From<Vec<Value>> for Value {
    fn from(x: Vec<Value>) -> Self {
        Self::Array(x)
    }
}

impl From<Map> for Value {
    fn from(x: Map) -> Self {
        Self::Map(x)
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => f.debug_tuple("Null").finish(),
            Value::String(x) => f.debug_tuple("String").field(x).finish(),
            Value::Bool(x) => f.debug_tuple("Bool").field(x).finish(),
            Value::I64(x) => f.debug_tuple("I64").field(x).finish(),
            Value::U64(x) => f.debug_tuple("U64").field(x).finish(),
            Value::F64(x) => f.debug_tuple("F64").field(x).finish(),
            Value::Decimal(x) => f.debug_tuple("Decimal").field(x).finish(),
            Value::I128(x) => f.debug_tuple("I128").field(x).finish(),
            Value::U128(x) => f.debug_tuple("U128").field(x).finish(),
            Value::Array(x) => f.debug_tuple("Array").field(x).finish(),
            Value::Map(x) => f.debug_tuple("Map").field(x).finish(),
            Value::Bytes(x) => f.debug_tuple("Bytes").field(&x.len()).finish(),
            Value::B32(x) => f
                .debug_tuple("B32")
                .field(&bs58::encode(x).into_string())
                .finish(),
            Value::B64(x) => f
                .debug_tuple("B64")
                .field(&bs58::encode(x).into_string())
                .finish(),
        }
    }
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("key must be a string")]
    KeyMustBeAString,
    #[error("invalid base58: {0}")]
    Bs58Decode(#[from] bs58::decode::Error),
    #[error("need length {need}, got {got}")]
    InvalidLenght { need: usize, got: usize },
    #[error("expected a map")]
    ExpectedMap,
    #[error("expected array")]
    ExpectedArray,
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

// default implementation of [u8] doesn't call serialize_bytes
pub(crate) struct Bytes<'a>(&'a [u8]);

impl<'a> serde::Serialize for Bytes<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.serialize_bytes(self.0)
    }
}

pub mod default {
    pub fn bool_true() -> bool {
        true
    }
}

pub(crate) struct OptionVisitor<V>(pub(crate) V);

impl<'de, V> serde::de::Visitor<'de> for OptionVisitor<V>
where
    V: serde::de::Visitor<'de>,
{
    type Value = Option<V::Value>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("optional ")?;
        self.0.expecting(formatter)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_any(self.0).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_solana_instruction() {
        use solana_sdk::instruction::{AccountMeta, Instruction};
        use solana_sdk::pubkey;

        let i = Instruction::new_with_bytes(
            pubkey!("ESxeViFP4r7THzVx9hJDkhj4HrNGSjJSFRPbGaAb97hN"),
            &[100; 1024],
            vec![AccountMeta {
                pubkey: pubkey!("ESxeViFP4r7THzVx9hJDkhj4HrNGSjJSFRPbGaAb97hN"),
                is_signer: true,
                is_writable: false,
            }],
        );

        let v = to_value(&i).unwrap();
        dbg!(&v);

        let i1: Instruction = from_value(v).unwrap();

        assert_eq!(i, i1);
    }

    #[test]
    fn test_json() {
        fn t(v: Value, s: &str) {
            assert_eq!(s, serde_json::to_string(&v).unwrap());
            assert_eq!(v, serde_json::from_str::<Value>(s).unwrap());
        }
        t(Value::Null, r#"{"N":0}"#);
        t(Value::String("hello".to_owned()), r#"{"S":"hello"}"#);
        t(Value::U64(0), r#"{"U":"0"}"#);
        t(Value::I64(-1), r#"{"I":"-1"}"#);
        t(
            Value::U128(u128::MAX),
            r#"{"U1":"340282366920938463463374607431768211455"}"#,
        );
        t(
            Value::I128(i128::MIN),
            r#"{"I1":"-170141183460469231731687303715884105728"}"#,
        );
        t(Value::Bool(true), r#"{"B":true}"#);
        t(
            Value::Decimal(dec!(3.1415926535897932384626433833)),
            r#"{"D":"3.1415926535897932384626433833"}"#,
        );
        t(
            crate::map! {
                "foo" => 1i64,
            }
            .into(),
            r#"{"M":{"foo":{"I":"1"}}}"#,
        );
        t(
            Value::Array(vec![1i64.into(), "hello".into()]),
            r#"{"A":[{"I":"1"},{"S":"hello"}]}"#,
        );
        t(
            Value::B32(
                bs58::decode("5sNRWMrT2P3KULzW3faaktCB3k2eqHow2GBJtcsCPcg7")
                    .into_vec()
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),
            r#"{"B3":"5sNRWMrT2P3KULzW3faaktCB3k2eqHow2GBJtcsCPcg7"}"#,
        );
        t(
            Value::B64(
                bs58::decode("3PvNxykqBz1BzBaq2AMU4Sa3CPJGnSC9JXkyzXe33m6W7Sj4MMgsZet6YxUQdPx1fEFU79QWm6RpPRVJAyeqiNsR")
                    .into_vec()
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),
            r#"{"B6":"3PvNxykqBz1BzBaq2AMU4Sa3CPJGnSC9JXkyzXe33m6W7Sj4MMgsZet6YxUQdPx1fEFU79QWm6RpPRVJAyeqiNsR"}"#,
        );
        t(
            Value::Bytes(bytes::Bytes::from_static(&[
                104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100,
            ])),
            r#"{"BY":"aGVsbG8gd29ybGQ="}"#,
        );
    }

    #[test]
    fn test_array_ser() {
        #[derive(serde::Serialize)]
        struct Output {
            value: Value,
        }

        let mut v = crate::to_map(&Output {
            value: Vec::from([Value::U64(1)]).into(),
        })
        .unwrap();
        assert_eq!(
            v.remove("value").unwrap(),
            Value::Array([1u64.into()].into())
        )
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_number_into_json() {
        let json: serde_json::Value = Value::Decimal(dec!(15966.2)).into();
        assert_eq!(json.as_f64().unwrap(), 15966.2);
    }
}
