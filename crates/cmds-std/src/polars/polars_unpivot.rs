use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_unpivot";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_unpivot.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub on: JsonValue,
    pub index: JsonValue,
    #[serde(default = "default_variable_name")]
    pub variable_name: String,
    #[serde(default = "default_value_name")]
    pub value_name: String,
}

fn default_variable_name() -> String { "variable".to_string() }
fn default_value_name() -> String { "value".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let on_cols = parse_column_names(&input.on)?;
    let index_cols = parse_column_names(&input.index)?;

    let on_smallstr: Vec<PlSmallStr> = on_cols.iter().map(|s| PlSmallStr::from(s.as_str())).collect();
    let index_smallstr: Vec<PlSmallStr> = index_cols.iter().map(|s| PlSmallStr::from(s.as_str())).collect();

    let args = UnpivotArgsIR {
        on: on_smallstr,
        index: index_smallstr,
        variable_name: Some(PlSmallStr::from(input.variable_name.as_str())),
        value_name: Some(PlSmallStr::from(input.value_name.as_str())),
    };

    let mut result = df
        .unpivot2(args)
        .map_err(|e| CommandError::msg(format!("Unpivot error: {e}")))?;

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
            Series::new("name".into(), &["Alice", "Bob"]).into_column(),
            Series::new("math".into(), &[90i64, 80]).into_column(),
            Series::new("science".into(), &[85i64, 95]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            on: serde_json::json!(["math", "science"]),
            index: serde_json::json!(["name"]),
            variable_name: "subject".to_string(),
            value_name: "score".to_string(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 4, "unpivot should produce 4 rows (2 names x 2 subjects)");
        assert!(df.column("name").is_ok(), "should have name column");
        assert!(df.column("subject").is_ok(), "should have subject column");
        assert!(df.column("score").is_ok(), "should have score column");
    }
}
