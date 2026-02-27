use flow_lib::command::prelude::*;
use rust_decimal::MathematicalOps;

const NAME: &str = "math";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("std/math.jsonc"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(with = "value::decimal")]
    a: Decimal,
    #[serde(with = "value::decimal")]
    b: Decimal,
    operator: String,
}

#[derive(Serialize, Debug)]
struct Output {
    result: Value,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    let a = input.a;
    let b = input.b;

    let result = match input.operator.trim() {
        "+" => a.checked_add(b).ok_or_else(|| CommandError::msg("addition overflow"))?,
        "-" => a.checked_sub(b).ok_or_else(|| CommandError::msg("subtraction overflow"))?,
        "*" => a.checked_mul(b).ok_or_else(|| CommandError::msg("multiplication overflow"))?,
        "/" => {
            if b.is_zero() {
                return Err(CommandError::msg("division by zero"));
            }
            a.checked_div(b)
                .ok_or_else(|| CommandError::msg("division overflow"))?
        }
        "%" => {
            if b.is_zero() {
                return Err(CommandError::msg("modulo by zero"));
            }
            a.checked_rem(b)
                .ok_or_else(|| CommandError::msg("modulo overflow"))?
        }
        "^" => a.powd(b),
        other => {
            return Err(CommandError::msg(format!(
                "unknown operator '{}', expected +, -, *, /, %, ^",
                other,
            )));
        }
    };

    Ok(Output {
        result: Value::Decimal(result),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(v: i64) -> Decimal {
        Decimal::new(v, 0)
    }

    fn dec_frac(v: i64, scale: u32) -> Decimal {
        Decimal::new(v, scale)
    }

    async fn eval(a: Value, op: &str, b: Value) -> Result<Decimal, CommandError> {
        let input = value::map! {
            "a" => a,
            "b" => b,
            "operator" => op,
        };
        let output = build().unwrap().run(<_>::default(), input).await?;
        match output.get("result") {
            Some(Value::Decimal(d)) => Ok(*d),
            other => panic!("expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_addition() {
        let r = eval(Value::U64(2), "+", Value::U64(3)).await.unwrap();
        assert_eq!(r, dec(5));
    }

    #[tokio::test]
    async fn test_subtraction() {
        let r = eval(Value::U64(10), "-", Value::U64(4)).await.unwrap();
        assert_eq!(r, dec(6));
    }

    #[tokio::test]
    async fn test_multiplication() {
        let r = eval(Value::U64(3), "*", Value::U64(7)).await.unwrap();
        assert_eq!(r, dec(21));
    }

    #[tokio::test]
    async fn test_division() {
        let r = eval(Value::U64(10), "/", Value::U64(4)).await.unwrap();
        assert_eq!(r, dec_frac(25, 1));
    }

    #[tokio::test]
    async fn test_modulo() {
        let r = eval(Value::U64(10), "%", Value::U64(3)).await.unwrap();
        assert_eq!(r, dec(1));
    }

    #[tokio::test]
    async fn test_exponentiation() {
        let r = eval(Value::U64(2), "^", Value::U64(10)).await.unwrap();
        assert_eq!(r, dec(1024));
    }

    #[tokio::test]
    async fn test_fractional_exponent() {
        // powd uses Taylor series approximation, so check within tolerance
        let r = eval(Value::U64(4), "^", Value::F64(0.5)).await.unwrap();
        assert!((r - dec(2)).abs() < dec_frac(1, 6)); // within 0.000001
    }

    #[tokio::test]
    async fn test_negative_exponent() {
        let r = eval(Value::U64(2), "^", Value::I64(-1)).await.unwrap();
        assert_eq!(r, dec_frac(5, 1));
    }

    #[tokio::test]
    async fn test_division_by_zero() {
        let err = eval(Value::U64(5), "/", Value::U64(0)).await.unwrap_err();
        assert!(err.to_string().contains("division by zero"));
    }

    #[tokio::test]
    async fn test_modulo_by_zero() {
        let err = eval(Value::U64(5), "%", Value::U64(0)).await.unwrap_err();
        assert!(err.to_string().contains("modulo by zero"));
    }

    #[tokio::test]
    async fn test_unknown_operator() {
        let err = eval(Value::U64(1), "??", Value::U64(1)).await.unwrap_err();
        assert!(err.to_string().contains("unknown operator"));
    }

    #[tokio::test]
    async fn test_mixed_types() {
        let r = eval(Value::I64(-3), "+", Value::String("7".to_string()))
            .await
            .unwrap();
        assert_eq!(r, dec(4));
    }

    #[tokio::test]
    async fn test_f64_division() {
        let r = eval(Value::F64(1.0), "/", Value::F64(3.0)).await.unwrap();
        // Decimal division of 1/3 is 0.3333...
        assert!(r > Decimal::ZERO);
        assert!(r < Decimal::ONE);
    }

    #[tokio::test]
    async fn test_string_inputs() {
        let r = eval(
            Value::String("123".to_string()),
            "*",
            Value::String("2".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(r, dec(246));
    }

    #[tokio::test]
    async fn test_non_numeric_string_error() {
        eval(Value::String("abc".to_string()), "+", Value::U64(1))
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn test_bool_error() {
        eval(Value::Bool(true), "+", Value::U64(1))
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn test_null_error() {
        eval(Value::U64(1), "+", Value::Null)
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn test_array_error() {
        eval(Value::Array(vec![Value::U64(1)]), "+", Value::U64(1))
            .await
            .unwrap_err();
    }
}
