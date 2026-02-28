use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_filter";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_filter.jsonc");

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
    pub operator: String,
    #[serde(default)]
    pub value: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

fn json_to_lit(value: &JsonValue) -> Expr {
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
        JsonValue::Null => lit(LiteralValue::Null),
        _ => lit(LiteralValue::Null),
    }
}

fn json_array_to_series(value: &JsonValue) -> Result<Series, CommandError> {
    match value {
        JsonValue::Array(arr) => {
            // Try to detect type from first non-null element
            let first_non_null = arr.iter().find(|v| !v.is_null());
            match first_non_null {
                Some(JsonValue::Number(n)) if n.is_i64() => {
                    let vals: Vec<Option<i64>> = arr
                        .iter()
                        .map(|v| v.as_i64())
                        .collect();
                    Ok(Series::new("filter_vals".into(), &vals))
                }
                Some(JsonValue::Number(_)) => {
                    let vals: Vec<Option<f64>> = arr
                        .iter()
                        .map(|v| v.as_f64())
                        .collect();
                    Ok(Series::new("filter_vals".into(), &vals))
                }
                Some(JsonValue::Bool(_)) => {
                    let vals: Vec<Option<bool>> = arr
                        .iter()
                        .map(|v| v.as_bool())
                        .collect();
                    Ok(Series::new("filter_vals".into(), &vals))
                }
                _ => {
                    let vals: Vec<Option<&str>> = arr
                        .iter()
                        .map(|v| v.as_str())
                        .collect();
                    Ok(Series::new("filter_vals".into(), &vals))
                }
            }
        }
        _ => Err(CommandError::msg("is_in operator requires a JSON array value")),
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let col_expr = col(&input.column);

    let filter_expr = match input.operator.as_str() {
        "eq" => col_expr.eq(json_to_lit(&input.value)),
        "neq" => col_expr.neq(json_to_lit(&input.value)),
        "gt" => col_expr.gt(json_to_lit(&input.value)),
        "gte" => col_expr.gt_eq(json_to_lit(&input.value)),
        "lt" => col_expr.lt(json_to_lit(&input.value)),
        "lte" => col_expr.lt_eq(json_to_lit(&input.value)),
        "is_null" => col_expr.is_null(),
        "is_not_null" => col_expr.is_not_null(),
        "contains" => {
            let pattern = input.value.as_str().unwrap_or("").to_string();
            col_expr.cast(DataType::String).str().contains_literal(lit(pattern))
        }
        "is_in" => {
            let series = json_array_to_series(&input.value)?;
            col_expr.is_in(lit(series))
        }
        other => {
            return Err(CommandError::msg(format!(
                "Unknown operator: {other}. Valid: eq, neq, gt, gte, lt, lte, is_null, is_not_null, contains, is_in"
            )));
        }
    };

    let mut result = df
        .lazy()
        .filter(filter_expr)
        .collect()
        .map_err(|e| CommandError::msg(format!("Filter error: {e}")))?;

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

    /// Build a small test DataFrame: name (str), age (i64), score (f64), active (bool)
    /// with one null row to exercise null-related operators.
    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[
                Some("Alice"), Some("Bob"), Some("Charlie"), None, Some("Eve"),
            ]).into_column(),
            Series::new("age".into(), &[
                Some(30i64), Some(25), Some(35), Some(28), None,
            ]).into_column(),
            Series::new("score".into(), &[
                Some(88.5f64), Some(92.0), Some(75.3), Some(88.5), Some(95.1),
            ]).into_column(),
            Series::new("active".into(), &[
                Some(true), Some(false), Some(true), Some(false), Some(true),
            ]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    /// Helper: run filter and return the resulting DataFrame.
    async fn filter(column: &str, operator: &str, value: JsonValue) -> DataFrame {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: column.into(),
            operator: operator.into(),
            value,
        }).await.unwrap();
        df_from_ipc(&output.dataframe).unwrap()
    }

    /// Helper: extract a string column from the result as Vec<Option<String>>.
    fn str_col(df: &DataFrame, name: &str) -> Vec<Option<String>> {
        df.column(name).unwrap()
            .as_materialized_series()
            .str().unwrap()
            .into_iter()
            .map(|v| v.map(|s| s.to_string()))
            .collect()
    }

    #[tokio::test]
    async fn test_eq_int() {
        let df = filter("age", "eq", serde_json::json!(30)).await;
        assert_eq!(df.height(), 1);
        assert_eq!(str_col(&df, "name"), vec![Some("Alice".into())]);
    }

    #[tokio::test]
    async fn test_eq_string() {
        let df = filter("name", "eq", serde_json::json!("Bob")).await;
        assert_eq!(df.height(), 1);
        assert_eq!(str_col(&df, "name"), vec![Some("Bob".into())]);
    }

    #[tokio::test]
    async fn test_neq() {
        let df = filter("age", "neq", serde_json::json!(30)).await;
        // age=25, 35, 28 match; null and 30 excluded
        assert_eq!(df.height(), 3);
    }

    #[tokio::test]
    async fn test_gt() {
        let df = filter("age", "gt", serde_json::json!(28)).await;
        // age 30 and 35 are > 28
        assert_eq!(df.height(), 2);
    }

    #[tokio::test]
    async fn test_gte() {
        let df = filter("age", "gte", serde_json::json!(30)).await;
        // age 30 and 35 are >= 30
        assert_eq!(df.height(), 2);
    }

    #[tokio::test]
    async fn test_lt() {
        let df = filter("score", "lt", serde_json::json!(80.0)).await;
        // score 75.3 is < 80
        assert_eq!(df.height(), 1);
        assert_eq!(str_col(&df, "name"), vec![Some("Charlie".into())]);
    }

    #[tokio::test]
    async fn test_lte() {
        let df = filter("score", "lte", serde_json::json!(88.5)).await;
        // 88.5, 75.3, 88.5 are <= 88.5
        assert_eq!(df.height(), 3);
    }

    #[tokio::test]
    async fn test_is_null() {
        let df = filter("name", "is_null", JsonValue::Null).await;
        assert_eq!(df.height(), 1);
        // The row where name is null has age=28
        let ages: Vec<Option<i64>> = df.column("age").unwrap()
            .as_materialized_series().i64().unwrap()
            .into_iter().collect();
        assert_eq!(ages, vec![Some(28)]);
    }

    #[tokio::test]
    async fn test_is_not_null() {
        let df = filter("age", "is_not_null", JsonValue::Null).await;
        // 4 rows have non-null age (Alice=30, Bob=25, Charlie=35, row4=28)
        assert_eq!(df.height(), 4);
    }

    #[tokio::test]
    async fn test_contains() {
        let df = filter("name", "contains", serde_json::json!("li")).await;
        // "Alice" and "Charlie" contain "li"
        assert_eq!(df.height(), 2);
        let names = str_col(&df, "name");
        assert!(names.contains(&Some("Alice".into())));
        assert!(names.contains(&Some("Charlie".into())));
    }

    #[tokio::test]
    async fn test_contains_literal_not_regex() {
        // Ensure regex metacharacters are treated literally, not as patterns
        let df = filter("name", "contains", serde_json::json!("A.*e")).await;
        // "A.*e" as a regex would match "Alice", but as a literal it matches nothing
        assert_eq!(df.height(), 0);
    }

    #[tokio::test]
    async fn test_is_in() {
        let df = filter("age", "is_in", serde_json::json!([25, 35])).await;
        // Bob (25) and Charlie (35)
        assert_eq!(df.height(), 2);
        let names = str_col(&df, "name");
        assert!(names.contains(&Some("Bob".into())));
        assert!(names.contains(&Some("Charlie".into())));
    }

    #[tokio::test]
    async fn test_is_in_strings() {
        let df = filter("name", "is_in", serde_json::json!(["Alice", "Eve"])).await;
        assert_eq!(df.height(), 2);
    }

    #[tokio::test]
    async fn test_eq_bool() {
        let df = filter("active", "eq", serde_json::json!(true)).await;
        // Alice, Charlie, Eve are active
        assert_eq!(df.height(), 3);
    }

    #[tokio::test]
    async fn test_unknown_operator_errors() {
        let result = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "age".into(),
            operator: "like".into(),
            value: serde_json::json!("foo"),
        }).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Unknown operator"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_filter_empty_result() {
        let df = filter("age", "gt", serde_json::json!(1000)).await;
        assert_eq!(df.height(), 0);
    }

    #[tokio::test]
    async fn test_output_json_present() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "age".into(),
            operator: "eq".into(),
            value: serde_json::json!(30),
        }).await.unwrap();
        // dataframe_json should be a non-null array
        assert!(output.dataframe_json.is_array());
        let arr = output.dataframe_json.as_array().unwrap();
        assert_eq!(arr.len(), 1); // one matching row
    }
}
