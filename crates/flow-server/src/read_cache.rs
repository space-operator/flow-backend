use actix_web::{
    ResponseError,
    http::{StatusCode, header::HttpDate},
};
use chrono::{DateTime, Utc};
use flow_lib::ValueSet;
use hashbrown::HashMap;
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::oneshot;
use value::Value;

use crate::error::Error;

#[derive(Clone)]
pub struct ReadCache {
    inner: Arc<Mutex<State>>,
    secret: [u8; blake3::KEY_LEN],
}

struct State {
    entries: HashMap<String, CacheEntry>,
    inflight: HashMap<String, Vec<oneshot::Sender<Result<CachedRead, CachedReadError>>>>,
}

#[derive(Clone)]
struct CacheEntry {
    read: CachedRead,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct CachedRead {
    pub body: Value,
    pub etag: String,
    pub cached_at: DateTime<Utc>,
    pub cache_control: String,
}

#[derive(Clone)]
struct CachedReadError {
    status: StatusCode,
    message: String,
}

impl ReadCache {
    pub fn new(secret: [u8; blake3::KEY_LEN]) -> Self {
        Self {
            inner: Arc::new(Mutex::new(State {
                entries: HashMap::new(),
                inflight: HashMap::new(),
            })),
            secret,
        }
    }

    pub fn make_request_key(
        &self,
        namespace: &str,
        target: &str,
        auth_scope: &str,
        inputs: &ValueSet,
    ) -> Result<String, Error> {
        let mut hasher = blake3::Hasher::new_keyed(&self.secret);
        hasher.update(namespace.as_bytes());
        hasher.update(&[0]);
        hasher.update(target.as_bytes());
        hasher.update(&[0]);
        hasher.update(auth_scope.as_bytes());
        hasher.update(&[0]);
        hasher.update(&canonical_json_bytes(inputs)?);
        Ok(hasher.finalize().to_hex().to_string())
    }

    pub fn build_cached_read(&self, body: Value, ttl: Duration) -> Result<CachedRead, Error> {
        let mut hasher = blake3::Hasher::new_keyed(&self.secret);
        hasher.update(&canonical_json_bytes(&body)?);
        let etag = format!("\"{}\"", hasher.finalize().to_hex());
        let cached_at = Utc::now();
        Ok(CachedRead {
            body,
            etag,
            cached_at,
            cache_control: format!("private, max-age={}", ttl.as_secs()),
        })
    }

    pub fn lookup(&self, key: &str) -> Option<CachedRead> {
        let mut state = self.inner.lock().unwrap();
        match state.entries.get(key) {
            Some(entry) if entry.expires_at > Instant::now() => Some(entry.read.clone()),
            Some(_) => {
                state.entries.remove(key);
                None
            }
            None => None,
        }
    }

    pub async fn get_or_compute<F, Fut>(
        &self,
        key: String,
        ttl: Duration,
        compute: F,
    ) -> Result<(CachedRead, bool), Error>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Value, Error>>,
    {
        if let Some(entry) = self.lookup(&key) {
            return Ok((entry, true));
        }

        let rx = {
            let mut state = self.inner.lock().unwrap();
            if let Some(entry) = state.entries.get(&key) {
                if entry.expires_at > Instant::now() {
                    return Ok((entry.read.clone(), true));
                }
                state.entries.remove(&key);
            }
            if let Some(waiters) = state.inflight.get_mut(&key) {
                let (tx, rx) = oneshot::channel();
                waiters.push(tx);
                Some(rx)
            } else {
                state.inflight.insert(key.clone(), Vec::new());
                None
            }
        };

        if let Some(rx) = rx {
            return match rx.await {
                Ok(Ok(entry)) => Ok((entry, true)),
                Ok(Err(error)) => Err(Error::custom(error.status, error.message)),
                Err(_) => Err(Error::custom(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "read cache waiter canceled",
                )),
            };
        }

        let computed = compute()
            .await
            .and_then(|body| self.build_cached_read(body, ttl));

        let result = match &computed {
            Ok(entry) => {
                let mut state = self.inner.lock().unwrap();
                state.entries.insert(
                    key.clone(),
                    CacheEntry {
                        read: entry.clone(),
                        expires_at: Instant::now() + ttl,
                    },
                );
                Ok(entry.clone())
            }
            Err(error) => Err(CachedReadError {
                status: error.status_code(),
                message: error.to_string(),
            }),
        };

        let waiters = {
            let mut state = self.inner.lock().unwrap();
            state.inflight.remove(&key).unwrap_or_default()
        };
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }

        match result {
            Ok(entry) => Ok((entry, false)),
            Err(error) => Err(Error::custom(error.status, error.message)),
        }
    }
}

impl CachedRead {
    pub fn last_modified(&self) -> String {
        let time = SystemTime::from(self.cached_at);
        HttpDate::from(time).to_string()
    }
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let json = serde_json::to_value(value).map_err(|error| {
        Error::custom(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to canonicalize read cache value: {error}"),
        )
    })?;
    serde_json::to_vec(&canonicalize_json(json)).map_err(|error| {
        Error::custom(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to serialize canonical read cache value: {error}"),
        )
    })
}

fn canonicalize_json(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Array(values) => {
            JsonValue::Array(values.into_iter().map(canonicalize_json).collect())
        }
        JsonValue::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let mut canonical = JsonMap::new();
            for (key, value) in entries {
                canonical.insert(key, canonicalize_json(value));
            }
            JsonValue::Object(canonical)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use value::map;

    #[test]
    fn request_keys_ignore_object_ordering() {
        let cache = ReadCache::new([7; blake3::KEY_LEN]);
        let first = map! {
            "a".to_owned() => Value::Map(map! {
                "z".to_owned() => Value::I64(1),
                "b".to_owned() => Value::Bool(true),
            }),
            "b".to_owned() => Value::Array(vec![Value::String("x".to_owned())]),
        };
        let second = map! {
            "b".to_owned() => Value::Array(vec![Value::String("x".to_owned())]),
            "a".to_owned() => Value::Map(map! {
                "b".to_owned() => Value::Bool(true),
                "z".to_owned() => Value::I64(1),
            }),
        };

        let first = cache
            .make_request_key("flow-read", "flow-1", "user:1", &first)
            .unwrap();
        let second = cache
            .make_request_key("flow-read", "flow-1", "user:1", &second)
            .unwrap();
        assert_eq!(first, second);
    }
}
