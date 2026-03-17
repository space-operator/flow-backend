use flow_lib::command::CommandError;
use serde::{Deserialize, Serialize};
use value::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse,
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    IsEmpty,
    IsNotEmpty,
    Contains,
}

fn value_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::U64(n) => Some(*n as f64),
        Value::I64(n) => Some(*n as f64),
        Value::F64(n) => Some(*n),
        Value::U128(n) => Some(*n as f64),
        Value::I128(n) => Some(*n as f64),
        Value::Decimal(d) => {
            use std::str::FromStr;
            f64::from_str(&d.to_string()).ok()
        }
        _ => None,
    }
}

fn resolve_field<'v>(value: &'v Value, field: Option<&str>) -> &'v Value {
    match field {
        Some(path) if !path.is_empty() => {
            let segments: Vec<&str> = path.split('.').collect();
            value::crud::get(value, &segments).unwrap_or(&Value::Null)
        }
        _ => value,
    }
}

pub fn evaluate(
    value: &Value,
    field: Option<&str>,
    operator: &Operator,
    compare_to: Option<&Value>,
) -> Result<bool, CommandError> {
    let resolved = resolve_field(value, field);

    match operator {
        Operator::IsNull => Ok(matches!(resolved, Value::Null)),
        Operator::IsNotNull => Ok(!matches!(resolved, Value::Null)),
        Operator::IsTrue => Ok(matches!(resolved, Value::Bool(true))),
        Operator::IsFalse => Ok(matches!(resolved, Value::Bool(false))),

        Operator::Eq => {
            let rhs = compare_to.unwrap_or(&Value::Null);
            Ok(resolved == rhs)
        }
        Operator::Ne => {
            let rhs = compare_to.unwrap_or(&Value::Null);
            Ok(resolved != rhs)
        }

        Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte => {
            let rhs = compare_to.ok_or_else(|| {
                CommandError::msg("compare_to is required for numeric comparison operators")
            })?;
            let lhs_f = value_to_f64(resolved).ok_or_else(|| {
                CommandError::msg(format!("cannot compare non-numeric value: {:?}", resolved))
            })?;
            let rhs_f = value_to_f64(rhs).ok_or_else(|| {
                CommandError::msg(format!(
                    "cannot compare to non-numeric value: {:?}",
                    rhs
                ))
            })?;
            Ok(match operator {
                Operator::Gt => lhs_f > rhs_f,
                Operator::Lt => lhs_f < rhs_f,
                Operator::Gte => lhs_f >= rhs_f,
                Operator::Lte => lhs_f <= rhs_f,
                _ => unreachable!(),
            })
        }

        Operator::IsEmpty => Ok(match resolved {
            Value::Null => true,
            Value::String(s) => s.is_empty(),
            Value::Array(a) => a.is_empty(),
            Value::Map(m) => m.is_empty(),
            _ => {
                return Err(CommandError::msg(format!(
                    "is_empty not supported for type: {:?}",
                    resolved
                )));
            }
        }),
        Operator::IsNotEmpty => {
            let empty = evaluate(value, field, &Operator::IsEmpty, None)?;
            Ok(!empty)
        }

        Operator::Contains => {
            let rhs = compare_to.ok_or_else(|| {
                CommandError::msg("compare_to is required for contains operator")
            })?;
            match resolved {
                Value::String(s) => match rhs {
                    Value::String(sub) => Ok(s.contains(sub.as_str())),
                    _ => Err(CommandError::msg(
                        "contains on string requires a string compare_to",
                    )),
                },
                Value::Array(arr) => Ok(arr.iter().any(|el| el == rhs)),
                Value::Map(m) => match rhs {
                    Value::String(key) => Ok(m.contains_key(key.as_str())),
                    _ => Err(CommandError::msg(
                        "contains on map requires a string key as compare_to",
                    )),
                },
                _ => Err(CommandError::msg(format!(
                    "contains not supported for type: {:?}",
                    resolved
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_checks() {
        assert!(evaluate(&Value::Null, None, &Operator::IsNull, None).unwrap());
        assert!(!evaluate(&Value::U64(1), None, &Operator::IsNull, None).unwrap());
        assert!(evaluate(&Value::U64(1), None, &Operator::IsNotNull, None).unwrap());
        assert!(!evaluate(&Value::Null, None, &Operator::IsNotNull, None).unwrap());
    }

    #[test]
    fn test_bool_checks() {
        assert!(evaluate(&Value::Bool(true), None, &Operator::IsTrue, None).unwrap());
        assert!(!evaluate(&Value::Bool(false), None, &Operator::IsTrue, None).unwrap());
        assert!(evaluate(&Value::Bool(false), None, &Operator::IsFalse, None).unwrap());
        assert!(!evaluate(&Value::Bool(true), None, &Operator::IsFalse, None).unwrap());
    }

    #[test]
    fn test_equality() {
        assert!(evaluate(
            &Value::String("hello".into()),
            None,
            &Operator::Eq,
            Some(&Value::String("hello".into()))
        )
        .unwrap());
        assert!(evaluate(
            &Value::U64(42),
            None,
            &Operator::Ne,
            Some(&Value::U64(43))
        )
        .unwrap());
        // eq with no compare_to compares to Null
        assert!(evaluate(&Value::Null, None, &Operator::Eq, None).unwrap());
    }

    #[test]
    fn test_numeric_comparisons() {
        assert!(evaluate(
            &Value::U64(10),
            None,
            &Operator::Gt,
            Some(&Value::U64(5))
        )
        .unwrap());
        assert!(evaluate(
            &Value::I64(-3),
            None,
            &Operator::Lt,
            Some(&Value::I64(0))
        )
        .unwrap());
        assert!(evaluate(
            &Value::F64(5.0),
            None,
            &Operator::Gte,
            Some(&Value::F64(5.0))
        )
        .unwrap());
        assert!(evaluate(
            &Value::U64(5),
            None,
            &Operator::Lte,
            Some(&Value::U64(5))
        )
        .unwrap());
        // cross-type numeric comparison
        assert!(evaluate(
            &Value::U64(10),
            None,
            &Operator::Gt,
            Some(&Value::I64(-1))
        )
        .unwrap());
    }

    #[test]
    fn test_numeric_comparison_errors() {
        // string is not numeric
        assert!(evaluate(
            &Value::String("abc".into()),
            None,
            &Operator::Gt,
            Some(&Value::U64(5))
        )
        .is_err());
        // missing compare_to
        assert!(evaluate(&Value::U64(5), None, &Operator::Gt, None).is_err());
    }

    #[test]
    fn test_empty_checks() {
        assert!(evaluate(&Value::String("".into()), None, &Operator::IsEmpty, None).unwrap());
        assert!(!evaluate(
            &Value::String("hello".into()),
            None,
            &Operator::IsEmpty,
            None
        )
        .unwrap());
        assert!(evaluate(&Value::Array(vec![]), None, &Operator::IsEmpty, None).unwrap());
        assert!(evaluate(&Value::Null, None, &Operator::IsEmpty, None).unwrap());
        assert!(evaluate(
            &Value::String("x".into()),
            None,
            &Operator::IsNotEmpty,
            None
        )
        .unwrap());
    }

    #[test]
    fn test_contains() {
        // string contains
        assert!(evaluate(
            &Value::String("hello world".into()),
            None,
            &Operator::Contains,
            Some(&Value::String("world".into()))
        )
        .unwrap());
        assert!(!evaluate(
            &Value::String("hello".into()),
            None,
            &Operator::Contains,
            Some(&Value::String("xyz".into()))
        )
        .unwrap());
        // array contains
        assert!(evaluate(
            &Value::Array(vec![Value::U64(1), Value::U64(2), Value::U64(3)]),
            None,
            &Operator::Contains,
            Some(&Value::U64(2))
        )
        .unwrap());
        // map contains key
        let map = Value::Map(value::map! {
            "name" => Value::String("Alice".into()),
            "age" => Value::U64(30),
        });
        assert!(evaluate(
            &map,
            None,
            &Operator::Contains,
            Some(&Value::String("name".into()))
        )
        .unwrap());
        assert!(!evaluate(
            &map,
            None,
            &Operator::Contains,
            Some(&Value::String("email".into()))
        )
        .unwrap());
        // map contains with non-string key errors
        assert!(evaluate(&map, None, &Operator::Contains, Some(&Value::U64(1))).is_err());
        // missing compare_to errors
        assert!(evaluate(
            &Value::String("hello".into()),
            None,
            &Operator::Contains,
            None
        )
        .is_err());
    }

    #[test]
    fn test_field_access() {
        let val = Value::Map(value::map! {
            "user" => Value::Map(value::map! {
                "name" => Value::String("Alice".into()),
                "age" => Value::U64(30),
            }),
        });
        assert!(evaluate(
            &val,
            Some("user.name"),
            &Operator::Eq,
            Some(&Value::String("Alice".into()))
        )
        .unwrap());
        assert!(evaluate(
            &val,
            Some("user.age"),
            &Operator::Gt,
            Some(&Value::U64(25))
        )
        .unwrap());
        // missing field resolves to Null
        assert!(evaluate(&val, Some("user.email"), &Operator::IsNull, None).unwrap());
    }

    #[test]
    fn test_array_index_in_field() {
        let val = Value::Map(value::map! {
            "items" => Value::Array(vec![
                Value::String("first".into()),
                Value::String("second".into()),
            ]),
        });
        assert!(evaluate(
            &val,
            Some("items.0"),
            &Operator::Eq,
            Some(&Value::String("first".into()))
        )
        .unwrap());
    }
}
