use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_filter_expr";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_filter_expr.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub expression: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

/// Parse a simple expression in the form "column operator value"
/// Supported operators: ==, !=, >, >=, <, <=, contains, is_null, is_not_null
fn parse_expression(expr: &str) -> Result<Expr, CommandError> {
    let expr = expr.trim();

    // Handle unary operators first
    if expr.ends_with("is_not_null") {
        let col_name = expr.trim_end_matches("is_not_null").trim();
        return Ok(col(col_name).is_not_null());
    }
    if expr.ends_with("is_null") {
        let col_name = expr.trim_end_matches("is_null").trim();
        return Ok(col(col_name).is_null());
    }

    // Try two-character operators first, then single-character.
    // Search for operators surrounded by whitespace to avoid mismatching
    // column names that contain operator characters (e.g., "score>=90").
    let operators = [">=", "<=", "!=", "==", ">", "<"];
    for op in &operators {
        let spaced = format!(" {} ", op);
        if let Some(pos) = expr.find(&spaced) {
            let col_name = expr[..pos].trim();
            let value_str = expr[pos + spaced.len()..].trim();
            let col_expr = col(col_name);
            let value_lit = parse_value_literal(value_str);

            return Ok(match *op {
                "==" => col_expr.eq(value_lit),
                "!=" => col_expr.neq(value_lit),
                ">" => col_expr.gt(value_lit),
                ">=" => col_expr.gt_eq(value_lit),
                "<" => col_expr.lt(value_lit),
                "<=" => col_expr.lt_eq(value_lit),
                _ => unreachable!(),
            });
        }
    }

    // Check for "contains" keyword
    if let Some(pos) = expr.find(" contains ") {
        let col_name = expr[..pos].trim();
        let pattern = expr[pos + " contains ".len()..].trim().trim_matches('"');
        return Ok(col(col_name).cast(DataType::String).str().contains_literal(lit(pattern.to_string())));
    }

    Err(CommandError::msg(format!(
        "Cannot parse expression: '{expr}'. Use format: 'column operator value' (e.g., 'age > 30', 'name == John')"
    )))
}

fn parse_value_literal(s: &str) -> Expr {
    let s = s.trim().trim_matches('"').trim_matches('\'');
    // Try integer
    if let Ok(i) = s.parse::<i64>() {
        return lit(i);
    }
    // Try float
    if let Ok(f) = s.parse::<f64>() {
        return lit(f);
    }
    // Try bool
    match s.to_lowercase().as_str() {
        "true" => return lit(true),
        "false" => return lit(false),
        "null" | "none" => return lit(LiteralValue::Null),
        _ => {}
    }
    // Default to string
    lit(s.to_string())
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let filter_expr = parse_expression(&input.expression)?;

    let mut result = df
        .lazy()
        .filter(filter_expr)
        .collect()
        .map_err(|e| CommandError::msg(format!("Filter expression error: {e}")))?;

    let (ipc, json) = dual_output(&mut result)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{df_to_ipc, df_from_ipc};

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), Some("Charlie"), Some("Alice")]).into_column(),
            Series::new("age".into(), &[Some(30i64), Some(25), Some(35), Some(30)]).into_column(),
            Series::new("score".into(), &[Some(88.5f64), Some(92.0), Some(75.3), Some(91.0)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    fn test_df_with_nulls_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), None, Some("Alice")]).into_column(),
            Series::new("age".into(), &[Some(30i64), None, Some(35), Some(30)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_gt() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            expression: "age > 28".into(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        // age 30, 35, 30 are > 28
        assert_eq!(df.height(), 3);
    }

    #[tokio::test]
    async fn test_run_eq_string() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            expression: "name == Alice".into(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        // Two rows with name "Alice"
        assert_eq!(df.height(), 2);
    }

    #[tokio::test]
    async fn test_run_contains() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            expression: "name contains li".into(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        // "Alice" (x2) and "Charlie" contain "li" -> 3 matches
        assert_eq!(df.height(), 3);
    }

    #[tokio::test]
    async fn test_run_is_null() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_with_nulls_ipc(),
            expression: "age is_null".into(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 1);
    }
}
