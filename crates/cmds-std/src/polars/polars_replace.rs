use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_replace";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_replace.jsonc");

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
    pub old: JsonValue,
    pub new: JsonValue,
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

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let old_lit = json_to_lit(&input.old);
    let new_lit = json_to_lit(&input.new);

    let mut result = df
        .lazy()
        .with_column(
            when(col(&input.column).eq(old_lit))
                .then(new_lit)
                .otherwise(col(&input.column))
                .alias(&input.column),
        )
        .collect()
        .map_err(|e| CommandError::msg(format!("Replace error: {e}")))?;

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

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
            Series::new("age".into(), &[30i64, 25, 35]).into_column(),
            Series::new("score".into(), &[88.5f64, 92.0, 75.3]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_replace_string() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "name".to_string(),
            old: serde_json::json!("Alice"),
            new: serde_json::json!("Alicia"),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let names = df.column("name").unwrap();
        assert_eq!(names.str().unwrap().get(0).unwrap(), "Alicia");
        assert_eq!(names.str().unwrap().get(1).unwrap(), "Bob");
    }
}
