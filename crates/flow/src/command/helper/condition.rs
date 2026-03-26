use flow_lib::command::CommandError;
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use value::{Decimal, Value};

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
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone)]
struct Rational {
    numer: BigInt,
    denom: BigInt,
}

impl Rational {
    fn integer(numer: impl Into<BigInt>) -> Self {
        Self {
            numer: numer.into(),
            denom: BigInt::from(1u8),
        }
    }

    fn decimal(decimal: Decimal) -> Self {
        let scale = decimal.scale();
        Self {
            numer: BigInt::from(decimal.mantissa()),
            denom: if scale == 0 {
                BigInt::from(1u8)
            } else {
                BigInt::from(10u8).pow(scale)
            },
        }
    }

    fn float(value: f64) -> Result<Self, CommandError> {
        if !value.is_finite() {
            return Err(CommandError::msg(format!(
                "cannot compare non-finite numeric value: {value:?}"
            )));
        }

        let bits = value.to_bits();
        let is_negative = (bits >> 63) != 0;
        let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
        let mantissa_bits = bits & ((1u64 << 52) - 1);

        if exponent_bits == 0 && mantissa_bits == 0 {
            return Ok(Self::integer(0));
        }

        let (significand, exponent) = if exponent_bits == 0 {
            (mantissa_bits, -1074)
        } else {
            ((1u64 << 52) | mantissa_bits, exponent_bits - 1023 - 52)
        };

        let mut numer = BigInt::from(significand);
        if is_negative {
            numer = -numer;
        }

        if exponent >= 0 {
            numer <<= exponent as usize;
            Ok(Self::integer(numer))
        } else {
            Ok(Self {
                numer,
                denom: BigInt::from(1u8) << (-exponent) as usize,
            })
        }
    }

    fn cmp(&self, rhs: &Self) -> Ordering {
        (&self.numer * &rhs.denom).cmp(&(&rhs.numer * &self.denom))
    }
}

fn numeric_value(v: &Value, context: &'static str) -> Result<Rational, CommandError> {
    match v {
        Value::U64(n) => Ok(Rational::integer(*n)),
        Value::I64(n) => Ok(Rational::integer(*n)),
        Value::F64(n) => Rational::float(*n),
        Value::U128(n) => Ok(Rational::integer(*n)),
        Value::I128(n) => Ok(Rational::integer(*n)),
        Value::Decimal(d) => Ok(Rational::decimal(*d)),
        _ => Err(CommandError::msg(format!("{context}: {:?}", v))),
    }
}

fn compare_numeric(lhs: &Value, rhs: &Value) -> Result<Ordering, CommandError> {
    let lhs = numeric_value(lhs, "cannot compare non-numeric value")?;
    let rhs = numeric_value(rhs, "cannot compare to non-numeric value")?;
    Ok(lhs.cmp(&rhs))
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
            let ordering = compare_numeric(resolved, rhs)?;
            Ok(match operator {
                Operator::Gt => ordering.is_gt(),
                Operator::Lt => ordering.is_lt(),
                Operator::Gte => !ordering.is_lt(),
                Operator::Lte => !ordering.is_gt(),
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
            let rhs = compare_to
                .ok_or_else(|| CommandError::msg("compare_to is required for contains operator"))?;
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

        Operator::StartsWith => {
            let rhs = compare_to.ok_or_else(|| {
                CommandError::msg("compare_to is required for starts_with operator")
            })?;
            match (resolved, rhs) {
                (Value::String(s), Value::String(prefix)) => Ok(s.starts_with(prefix.as_str())),
                (Value::String(_), _) => Err(CommandError::msg(
                    "starts_with on string requires a string compare_to",
                )),
                _ => Err(CommandError::msg(format!(
                    "starts_with not supported for type: {:?}",
                    resolved
                ))),
            }
        }

        Operator::EndsWith => {
            let rhs = compare_to.ok_or_else(|| {
                CommandError::msg("compare_to is required for ends_with operator")
            })?;
            match (resolved, rhs) {
                (Value::String(s), Value::String(suffix)) => Ok(s.ends_with(suffix.as_str())),
                (Value::String(_), _) => Err(CommandError::msg(
                    "ends_with on string requires a string compare_to",
                )),
                _ => Err(CommandError::msg(format!(
                    "ends_with not supported for type: {:?}",
                    resolved
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
        assert!(
            evaluate(
                &Value::String("hello".into()),
                None,
                &Operator::Eq,
                Some(&Value::String("hello".into()))
            )
            .unwrap()
        );
        assert!(evaluate(&Value::U64(42), None, &Operator::Ne, Some(&Value::U64(43))).unwrap());
        // eq with no compare_to compares to Null
        assert!(evaluate(&Value::Null, None, &Operator::Eq, None).unwrap());
    }

    #[test]
    fn test_numeric_comparisons() {
        assert!(evaluate(&Value::U64(10), None, &Operator::Gt, Some(&Value::U64(5))).unwrap());
        assert!(evaluate(&Value::I64(-3), None, &Operator::Lt, Some(&Value::I64(0))).unwrap());
        assert!(
            evaluate(
                &Value::F64(5.0),
                None,
                &Operator::Gte,
                Some(&Value::F64(5.0))
            )
            .unwrap()
        );
        assert!(evaluate(&Value::U64(5), None, &Operator::Lte, Some(&Value::U64(5))).unwrap());
        // cross-type numeric comparison
        assert!(evaluate(&Value::U64(10), None, &Operator::Gt, Some(&Value::I64(-1))).unwrap());
    }

    #[test]
    fn test_numeric_comparisons_do_not_lose_u64_precision() {
        let lhs = Value::U64(9_007_199_254_740_993);
        let rhs = Value::U64(9_007_199_254_740_992);
        assert!(evaluate(&lhs, None, &Operator::Gt, Some(&rhs)).unwrap());
    }

    #[test]
    fn test_numeric_comparisons_do_not_lose_u128_precision() {
        let lhs = Value::U128(u128::MAX);
        let rhs = Value::U128(u128::MAX - 1);
        assert!(evaluate(&lhs, None, &Operator::Gt, Some(&rhs)).unwrap());
    }

    #[test]
    fn test_decimal_comparisons_are_exact() {
        let lhs = Value::Decimal(Decimal::from_str("0.1000000000000000001").unwrap());
        let rhs = Value::Decimal(Decimal::from_str("0.1").unwrap());
        assert!(evaluate(&lhs, None, &Operator::Gt, Some(&rhs)).unwrap());
    }

    #[test]
    fn test_float_vs_integer_comparisons_use_exact_float_value() {
        let lhs = Value::F64(9_007_199_254_740_992.0);
        let rhs = Value::U64(9_007_199_254_740_993);
        assert!(evaluate(&lhs, None, &Operator::Lt, Some(&rhs)).unwrap());
    }

    #[test]
    fn test_numeric_comparison_errors() {
        // string is not numeric
        assert!(
            evaluate(
                &Value::String("abc".into()),
                None,
                &Operator::Gt,
                Some(&Value::U64(5))
            )
            .is_err()
        );
        // missing compare_to
        assert!(evaluate(&Value::U64(5), None, &Operator::Gt, None).is_err());
        // non-finite float
        assert!(
            evaluate(
                &Value::F64(f64::NAN),
                None,
                &Operator::Gt,
                Some(&Value::U64(1))
            )
            .is_err()
        );
        // wrong compare_to type for string operators
        assert!(
            evaluate(
                &Value::String("hello".into()),
                None,
                &Operator::StartsWith,
                Some(&Value::U64(1))
            )
            .is_err()
        );
        assert!(
            evaluate(
                &Value::String("hello".into()),
                None,
                &Operator::EndsWith,
                Some(&Value::U64(1))
            )
            .is_err()
        );
    }

    #[test]
    fn test_empty_checks() {
        assert!(evaluate(&Value::String("".into()), None, &Operator::IsEmpty, None).unwrap());
        assert!(
            !evaluate(
                &Value::String("hello".into()),
                None,
                &Operator::IsEmpty,
                None
            )
            .unwrap()
        );
        assert!(evaluate(&Value::Array(vec![]), None, &Operator::IsEmpty, None).unwrap());
        assert!(evaluate(&Value::Null, None, &Operator::IsEmpty, None).unwrap());
        assert!(
            evaluate(
                &Value::String("x".into()),
                None,
                &Operator::IsNotEmpty,
                None
            )
            .unwrap()
        );
    }

    #[test]
    fn test_contains() {
        // string contains
        assert!(
            evaluate(
                &Value::String("hello world".into()),
                None,
                &Operator::Contains,
                Some(&Value::String("world".into()))
            )
            .unwrap()
        );
        assert!(
            !evaluate(
                &Value::String("hello".into()),
                None,
                &Operator::Contains,
                Some(&Value::String("xyz".into()))
            )
            .unwrap()
        );
        // array contains
        assert!(
            evaluate(
                &Value::Array(vec![Value::U64(1), Value::U64(2), Value::U64(3)]),
                None,
                &Operator::Contains,
                Some(&Value::U64(2))
            )
            .unwrap()
        );
        // map contains key
        let map = Value::Map(value::map! {
            "name" => Value::String("Alice".into()),
            "age" => Value::U64(30),
        });
        assert!(
            evaluate(
                &map,
                None,
                &Operator::Contains,
                Some(&Value::String("name".into()))
            )
            .unwrap()
        );
        assert!(
            !evaluate(
                &map,
                None,
                &Operator::Contains,
                Some(&Value::String("email".into()))
            )
            .unwrap()
        );
        // map contains with non-string key errors
        assert!(evaluate(&map, None, &Operator::Contains, Some(&Value::U64(1))).is_err());
        // missing compare_to errors
        assert!(
            evaluate(
                &Value::String("hello".into()),
                None,
                &Operator::Contains,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn test_starts_with() {
        assert!(
            evaluate(
                &Value::String("hello world".into()),
                None,
                &Operator::StartsWith,
                Some(&Value::String("hello".into()))
            )
            .unwrap()
        );
        assert!(
            !evaluate(
                &Value::String("hello world".into()),
                None,
                &Operator::StartsWith,
                Some(&Value::String("world".into()))
            )
            .unwrap()
        );
        assert!(
            evaluate(
                &Value::String("hello world".into()),
                Some(""),
                &Operator::StartsWith,
                Some(&Value::String("hello".into()))
            )
            .unwrap()
        );
    }

    #[test]
    fn test_ends_with() {
        assert!(
            evaluate(
                &Value::String("hello world".into()),
                None,
                &Operator::EndsWith,
                Some(&Value::String("world".into()))
            )
            .unwrap()
        );
        assert!(
            !evaluate(
                &Value::String("hello world".into()),
                None,
                &Operator::EndsWith,
                Some(&Value::String("hello".into()))
            )
            .unwrap()
        );
    }

    #[test]
    fn test_field_access() {
        let val = Value::Map(value::map! {
            "user" => Value::Map(value::map! {
                "name" => Value::String("Alice".into()),
                "age" => Value::U64(30),
            }),
        });
        assert!(
            evaluate(
                &val,
                Some("user.name"),
                &Operator::Eq,
                Some(&Value::String("Alice".into()))
            )
            .unwrap()
        );
        assert!(evaluate(&val, Some("user.age"), &Operator::Gt, Some(&Value::U64(25))).unwrap());
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
        assert!(
            evaluate(
                &val,
                Some("items.0"),
                &Operator::Eq,
                Some(&Value::String("first".into()))
            )
            .unwrap()
        );
    }
}
