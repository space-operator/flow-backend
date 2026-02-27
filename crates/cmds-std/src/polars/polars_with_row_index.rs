use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_with_row_index";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_with_row_index.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub offset: u32,
}

fn default_name() -> String { "index".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    let mut result = df
        .with_row_index(PlSmallStr::from(input.name.as_str()), Some(input.offset))
        .map_err(|e| CommandError::msg(format!("With row index error: {e}")))?;

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
    async fn test_run_with_row_index_default() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            name: "index".to_string(),
            offset: 0,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 4);
        let idx = df.column("index").unwrap();
        let idx_vals: Vec<u32> = idx.u32().unwrap().into_no_null_iter().collect();
        assert_eq!(idx_vals, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_run_with_row_index_custom() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            name: "row_num".to_string(),
            offset: 10,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert!(df.column("row_num").is_ok());
        let idx = df.column("row_num").unwrap();
        let idx_vals: Vec<u32> = idx.u32().unwrap().into_no_null_iter().collect();
        assert_eq!(idx_vals, vec![10, 11, 12]);
    }
}
