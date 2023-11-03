use crate::Value;
use thiserror::Error as ThisError;

pub mod path;

fn parse_index(key: &str) -> Option<usize> {
    if key == "0" {
        Some(0)
    } else if key.starts_with('0') {
        None
    } else {
        key.parse().ok()
    }
}

/// [evaluation](https://www.rfc-editor.org/rfc/rfc6901#section-4) operation
pub fn get<'v, S: AsRef<str>>(value: &'v Value, path: &[S]) -> Option<&'v Value> {
    let mut result = value;
    for s in path {
        let key = s.as_ref();
        result = match result {
            Value::Map(map) => match map.get(key) {
                Some(value) => value,
                None => return None,
            },
            Value::Array(array) => {
                let idx = parse_index(key)?;
                array.get(idx)?
            }
            _ => return None,
        };
    }
    Some(result)
}

/// [evaluation](https://www.rfc-editor.org/rfc/rfc6901#section-4) operation
pub fn get_mut<'v, S: AsRef<str>>(value: &'v mut Value, path: &[S]) -> Option<&'v mut Value> {
    let mut result = value;
    for s in path {
        let key = s.as_ref();
        result = match result {
            Value::Map(map) => match map.get_mut(key) {
                Some(value) => value,
                None => return None,
            },
            Value::Array(array) => {
                let idx = parse_index(key)?;
                array.get_mut(idx)?
            }
            _ => return None,
        };
    }
    Some(result)
}

/// [remove](https://www.rfc-editor.org/rfc/rfc6902#section-4.2) operation
pub fn remove<S: AsRef<str>>(value: &mut Value, path: &[S]) -> Option<Value> {
    if path.is_empty() {
        return Some(std::mem::replace(value, Value::Null));
    }

    let parent_path = &path[..path.len() - 1];

    let parent = get_mut(value, parent_path)?;
    let key = path.last().expect("!path.is_empty()").as_ref();
    match parent {
        Value::Map(map) => map.remove(key),
        Value::Array(array) => {
            let idx = parse_index(key)?;
            if idx < array.len() {
                Some(array.remove(idx))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(ThisError, Debug)]
#[error("failed to insert")]
pub struct InsertError;

/// [add](https://www.rfc-editor.org/rfc/rfc6902#section-4.1) operation
pub fn insert<S: AsRef<str>>(
    value: &mut Value,
    path: &[S],
    insert: Value,
) -> Result<Option<Value>, InsertError> {
    if path.is_empty() {
        return Ok(Some(std::mem::replace(value, insert)));
    }

    let parent_path = &path[..path.len() - 1];
    let parent = get_mut(value, parent_path).ok_or(InsertError)?;
    let key = path.last().expect("!path.is_empty()").as_ref();
    match parent {
        Value::Map(map) => Ok(map.insert(key.to_owned(), insert)),
        Value::Array(array) => {
            if key == "-" {
                array.push(insert);
            } else {
                let idx = parse_index(key).ok_or(InsertError)?;
                if idx > array.len() {
                    return Err(InsertError);
                } else {
                    array.insert(idx, insert);
                }
            }
            Ok(None)
        }
        _ => Err(InsertError),
    }
}
