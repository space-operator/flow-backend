use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_fill_null";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_fill_null.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub column: String,
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(default)]
    pub value: JsonValue,
}

fn default_strategy() -> String { "forward".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

fn json_to_fill_expr(value: &JsonValue) -> Expr {
    match value {
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                lit(i)
            } else if let Some(f) = n.as_f64() {
                lit(f)
            } else {
                lit(LiteralValue::Null)
            }
        }
        JsonValue::String(s) => lit(s.clone()),
        JsonValue::Bool(b) => lit(*b),
        _ => lit(LiteralValue::Null),
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    let fill_col = match input.strategy.to_lowercase().as_str() {
        "forward" => col(&input.column).forward_fill(None),
        "backward" => col(&input.column).backward_fill(None),
        "mean" => col(&input.column).fill_null(col(&input.column).mean()),
        "min" => col(&input.column).fill_null(col(&input.column).min()),
        "max" => col(&input.column).fill_null(col(&input.column).max()),
        "zero" => col(&input.column).fill_null(lit(0)),
        "one" => col(&input.column).fill_null(lit(1)),
        "value" => col(&input.column).fill_null(json_to_fill_expr(&input.value)),
        other => {
            return Err(CommandError::msg(format!(
                "Unknown fill_null strategy: {other}. Valid: forward, backward, mean, min, max, zero, one, value"
            )));
        }
    };

    let mut result = df
        .lazy()
        .with_column(fill_col)
        .collect()
        .map_err(|e| CommandError::msg(format!("Fill null error: {e}")))?;

    let (ipc, json) = dual_output(&mut result)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;

    #[test]
    fn test_build() { build().unwrap(); }

    fn df_with_nulls_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
            Series::new("age".into(), &[Some(30i64), None, Some(35)]).into_column(),
            Series::new("score".into(), &[88.5f64, 92.0, 75.3]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_fill_null_value() {
        let output = run(CommandContext::default(), Input {
            dataframe: df_with_nulls_ipc(),
            column: "age".to_string(),
            strategy: "value".to_string(),
            value: serde_json::json!(99),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let ages = df.column("age").unwrap();
        assert_eq!(ages.null_count(), 0);
        assert_eq!(ages.i64().unwrap().get(1).unwrap(), 99);
    }

    #[tokio::test]
    async fn test_run_fill_null_forward() {
        let output = run(CommandContext::default(), Input {
            dataframe: df_with_nulls_ipc(),
            column: "age".to_string(),
            strategy: "forward".to_string(),
            value: serde_json::json!(null),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let ages = df.column("age").unwrap();
        assert_eq!(ages.null_count(), 0);
        // Forward fill: Bob's null gets Alice's value (30)
        assert_eq!(ages.i64().unwrap().get(1).unwrap(), 30);
    }
}
