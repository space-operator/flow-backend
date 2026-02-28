//! In-memory TTL cache for DAS API responses.
//!
//! Caches the JSON-RPC `result` value for 30 seconds to avoid
//! redundant RPC calls for the same query within a flow.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(30);
const MAX_ENTRIES: usize = 1000;

struct Entry {
    value: JsonValue,
    expires_at: Instant,
}

static CACHE: LazyLock<Mutex<HashMap<String, Entry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Build a cache key from DAS method name and its params object.
pub fn key(method: &str, params: &JsonValue) -> String {
    format!("{}:{}", method, params)
}

/// Get a cached result, returning `None` on miss or expiry.
pub fn get(key: &str) -> Option<JsonValue> {
    let mut map = CACHE.lock().ok()?;
    match map.get(key) {
        Some(entry) if entry.expires_at > Instant::now() => Some(entry.value.clone()),
        Some(_) => {
            map.remove(key);
            None
        }
        None => None,
    }
}

/// Store a result in cache with 30s TTL.
pub fn set(key: String, value: JsonValue) {
    if let Ok(mut map) = CACHE.lock() {
        if map.len() > MAX_ENTRIES {
            let now = Instant::now();
            map.retain(|_, e| e.expires_at > now);
        }
        map.insert(key, Entry {
            value,
            expires_at: Instant::now() + TTL,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_hit() {
        let k = key("getAsset", &json!({"id": "abc123"}));
        let val = json!({"name": "test"});
        set(k.clone(), val.clone());
        assert_eq!(get(&k), Some(val));
    }

    #[test]
    fn test_cache_miss() {
        assert_eq!(get("nonexistent:key"), None);
    }

    #[test]
    fn test_cache_key_format() {
        let k = key("getAsset", &json!({"id": "abc123"}));
        assert_eq!(k, r#"getAsset:{"id":"abc123"}"#);
    }
}
