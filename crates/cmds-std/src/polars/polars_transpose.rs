use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_transpose";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_transpose.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    #[serde(default = "default_true")]
    pub include_header: bool,
    #[serde(default = "default_header_name")]
    pub header_name: String,
}

fn default_true() -> bool { true }
fn default_header_name() -> String { "column".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_ipc(&input.dataframe)?;

    let mut result = if input.include_header {
        df.transpose(Some(input.header_name.as_str()), None)
            .map_err(|e| CommandError::msg(format!("Transpose error: {e}")))?
    } else {
        df.transpose(None, None)
            .map_err(|e| CommandError::msg(format!("Transpose error: {e}")))?
    };

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
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("a".into(), &[1i64, 4]).into_column(),
            Series::new("b".into(), &[2i64, 5]).into_column(),
            Series::new("c".into(), &[3i64, 6]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_with_header() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            include_header: true,
            header_name: "column".to_string(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        // 2 rows, 3 columns => transposed: 3 rows, 2 data cols + 1 header col = 3 cols
        assert_eq!(df.height(), 3, "transpose should produce 3 rows (one per original column)");
        assert!(df.column("column").is_ok(), "should have header column");
    }

    #[tokio::test]
    async fn test_run_without_header() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            include_header: false,
            header_name: "column".to_string(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3, "transpose should produce 3 rows");
        // Without header, should have 2 columns (one per original row)
        assert_eq!(df.width(), 2, "transpose without header should have 2 columns");
    }
}
