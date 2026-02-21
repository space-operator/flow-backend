use flow_lib::command::prelude::*;

const NAME: &str = "range";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("std/range.jsonc"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(with = "value::decimal")]
    start: Decimal,
    #[serde(with = "value::decimal")]
    end: Decimal,
    #[serde(default, with = "value::decimal::opt")]
    step_by: Option<Decimal>,
}

#[derive(Serialize, Debug)]
struct Output {
    result: Vec<Value>,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    const MAX_LENGTH: usize = 10_000_000;

    let start = input.start;
    let end = input.end;

    if start == end {
        return Ok(Output { result: vec![] });
    }

    let step = input
        .step_by
        .map(|s| s.abs())
        .unwrap_or(Decimal::ONE);

    if step.is_zero() {
        return Err(CommandError::msg("step must not be zero"));
    }

    let diff = (end - start).abs();
    let length: usize = (diff / step).ceil().try_into()?;

    if length > MAX_LENGTH {
        return Err(CommandError::msg(format!(
            "range would produce {} elements, maximum is {}",
            length, MAX_LENGTH,
        )));
    }

    let signed_step = if start < end { step } else { -step };
    let mut current = start;
    let mut result = Vec::with_capacity(length);

    for _ in 0..length {
        result.push(Value::Decimal(current));
        current += signed_step;
    }

    Ok(Output { result })
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

    fn to_decimals(output: &Output) -> Vec<Decimal> {
        output
            .result
            .iter()
            .map(|v| match v {
                Value::Decimal(d) => *d,
                _ => panic!("expected Decimal, got {:?}", v),
            })
            .collect()
    }

    async fn eval(start: Value, end: Value, step_by: Option<Value>) -> Result<Output, CommandError> {
        let mut input = value::map! {
            "start" => start,
            "end" => end,
        };
        if let Some(s) = step_by {
            input.insert("step_by".to_string(), s);
        }
        let output = build().unwrap().run(<_>::default(), input).await?;
        Ok(Output {
            result: match output.get("result") {
                Some(Value::Array(a)) => a.clone(),
                other => panic!("expected Array, got {:?}", other),
            },
        })
    }

    #[tokio::test]
    async fn test_basic_ascending() {
        let output = run(
            <_>::default(),
            Input {
                start: dec(0),
                end: dec(5),
                step_by: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(to_decimals(&output), vec![dec(0), dec(1), dec(2), dec(3), dec(4)]);
    }

    #[tokio::test]
    async fn test_descending() {
        let output = run(
            <_>::default(),
            Input {
                start: dec(5),
                end: dec(0),
                step_by: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(to_decimals(&output), vec![dec(5), dec(4), dec(3), dec(2), dec(1)]);
    }

    #[tokio::test]
    async fn test_step_by() {
        let output = run(
            <_>::default(),
            Input {
                start: dec(0),
                end: dec(5),
                step_by: Some(dec(2)),
            },
        )
        .await
        .unwrap();
        assert_eq!(to_decimals(&output), vec![dec(0), dec(2), dec(4)]);
    }

    #[tokio::test]
    async fn test_fractional_step() {
        let output = run(
            <_>::default(),
            Input {
                start: dec(0),
                end: dec(1),
                step_by: Some(dec_frac(3, 1)), // 0.3
            },
        )
        .await
        .unwrap();
        assert_eq!(
            to_decimals(&output),
            vec![dec(0), dec_frac(3, 1), dec_frac(6, 1), dec_frac(9, 1)]
        );
    }

    #[tokio::test]
    async fn test_descending_with_positive_step() {
        let output = run(
            <_>::default(),
            Input {
                start: dec(3),
                end: dec(0),
                step_by: Some(dec(1)),
            },
        )
        .await
        .unwrap();
        assert_eq!(to_decimals(&output), vec![dec(3), dec(2), dec(1)]);
    }

    #[tokio::test]
    async fn test_equal_start_end() {
        let output = run(
            <_>::default(),
            Input {
                start: dec(5),
                end: dec(5),
                step_by: None,
            },
        )
        .await
        .unwrap();
        assert!(output.result.is_empty());
    }

    #[tokio::test]
    async fn test_zero_step_error() {
        let err = run(
            <_>::default(),
            Input {
                start: dec(0),
                end: dec(5),
                step_by: Some(Decimal::ZERO),
            },
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("step must not be zero"));
    }

    #[tokio::test]
    async fn test_u64_input() {
        let output = eval(Value::U64(0), Value::U64(3), None).await.unwrap();
        assert_eq!(to_decimals(&output), vec![dec(0), dec(1), dec(2)]);
    }

    #[tokio::test]
    async fn test_i64_input() {
        let output = eval(Value::I64(-2), Value::I64(2), None).await.unwrap();
        assert_eq!(to_decimals(&output), vec![dec(-2), dec(-1), dec(0), dec(1)]);
    }

    #[tokio::test]
    async fn test_f64_input() {
        let output = eval(Value::F64(0.0), Value::F64(1.0), Some(Value::F64(0.5)))
            .await
            .unwrap();
        assert_eq!(to_decimals(&output), vec![dec(0), dec_frac(5, 1)]);
    }

    #[tokio::test]
    async fn test_string_input() {
        let output = eval(
            Value::String("0".to_string()),
            Value::String("3".to_string()),
            None,
        )
        .await
        .unwrap();
        assert_eq!(to_decimals(&output), vec![dec(0), dec(1), dec(2)]);
    }

    #[tokio::test]
    async fn test_non_numeric_error() {
        eval(Value::Bool(true), Value::U64(5), None)
            .await
            .unwrap_err();
    }
}
