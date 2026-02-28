use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_str_operations";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_str_operations.jsonc");

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
    pub operation: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub replacement: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let pattern = input.pattern.unwrap_or_default();
    let replacement = input.replacement.unwrap_or_default();

    let (expr, output_col) = match input.operation.as_str() {
        "contains" => (
            col(&input.column).str().contains_literal(lit(pattern)),
            format!("{}_result", input.column),
        ),
        "starts_with" => (
            col(&input.column).str().starts_with(lit(pattern)),
            format!("{}_result", input.column),
        ),
        "ends_with" => (
            col(&input.column).str().ends_with(lit(pattern)),
            format!("{}_result", input.column),
        ),
        "replace" => (
            col(&input.column).str().replace(lit(pattern), lit(replacement), true),
            input.column.clone(),
        ),
        "to_lowercase" => (
            col(&input.column).str().to_lowercase(),
            input.column.clone(),
        ),
        "to_uppercase" => (
            col(&input.column).str().to_uppercase(),
            input.column.clone(),
        ),
        "strip" | "trim" => (
            col(&input.column).str().strip_chars(lit(LiteralValue::Null)),
            input.column.clone(),
        ),
        "len" | "lengths" => (
            col(&input.column).str().len_chars(),
            format!("{}_len", input.column),
        ),
        other => return Err(CommandError::msg(format!(
            "Unknown string operation: '{other}'. Use: contains, starts_with, ends_with, replace, to_lowercase, to_uppercase, strip, len"
        ))),
    };

    let mut result = df
        .lazy()
        .with_column(expr.alias(&output_col))
        .collect()
        .map_err(|e| CommandError::msg(format!("String operation error: {e}")))?;

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

    fn test_str_df_ipc(col_name: &str, values: &[&str]) -> String {
        let s = Series::new(col_name.into(), values);
        let mut df = DataFrame::new(vec![s.into_column()]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_to_lowercase() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_str_df_ipc("names", &["Alice", "Bob", "CHARLIE"]),
            column: "names".to_string(),
            operation: "to_lowercase".to_string(),
            pattern: None,
            replacement: None,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let col = df.column("names").unwrap();
        let vals: Vec<&str> = col.str().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec!["alice", "bob", "charlie"]);
    }

    #[tokio::test]
    async fn test_run_to_uppercase() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_str_df_ipc("names", &["Alice", "Bob", "CHARLIE"]),
            column: "names".to_string(),
            operation: "to_uppercase".to_string(),
            pattern: None,
            replacement: None,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let col = df.column("names").unwrap();
        let vals: Vec<&str> = col.str().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec!["ALICE", "BOB", "CHARLIE"]);
    }

    #[tokio::test]
    async fn test_run_contains() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_str_df_ipc("names", &["Alice", "Bob", "CHARLIE"]),
            column: "names".to_string(),
            operation: "contains".to_string(),
            pattern: Some("li".to_string()),
            replacement: None,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let col = df.column("names_result").unwrap();
        let vals: Vec<bool> = col.bool().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![true, false, false]);
    }

    #[tokio::test]
    async fn test_run_len() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_str_df_ipc("names", &["Alice", "Bob", "CHARLIE"]),
            column: "names".to_string(),
            operation: "len".to_string(),
            pattern: None,
            replacement: None,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let col = df.column("names_len").unwrap();
        let vals: Vec<u32> = col.u32().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![5, 3, 7]);
    }
}
