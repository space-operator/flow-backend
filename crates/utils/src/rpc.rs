use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use std::borrow::Cow;

pub struct V2;

impl Serialize for V2 {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        "2.0".serialize(s)
    }
}

impl<'de> Deserialize<'de> for V2 {
    fn deserialize<D>(d: D) -> Result<V2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(d)?;
        if s == "2.0" {
            Ok(V2)
        } else {
            Err(serde::de::Error::custom("Could not deserialize V2"))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request<R> {
    pub jsonrpc: V2,
    #[serde(flatten)]
    pub request: R,
    pub id: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response<T, E> {
    Ok {
        jsonrpc: V2,
        result: T,
        id: i64,
    },
    Err {
        jsonrpc: V2,
        error: RpcError<E>,
        id: i64,
    },
}

#[derive(Serialize, Deserialize)]
pub struct RpcError<E = ()> {
    pub code: i64,
    pub message: String,
    pub data: Option<E>,
}

#[derive(Serialize, Deserialize)]
pub struct Notification<'a, T> {
    pub jsonrpc: V2,
    pub method: Cow<'a, str>,
    pub params: T,
}
