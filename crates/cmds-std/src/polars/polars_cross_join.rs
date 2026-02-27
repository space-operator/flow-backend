use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_cross_join";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_cross_join.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub left: String,
    pub right: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let left = df_from_ipc(&input.left)?;
    let right = df_from_ipc(&input.right)?;

    let mut result = left
        .cross_join(&right, None, None)
        .map_err(|e| CommandError::msg(format!("Cross join error: {e}")))?;

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
    fn test_build() {
        build().unwrap();
    }

    fn left_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("id".into(), &[1i64, 2, 3]).into_column(),
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    fn right_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("id".into(), &[2i64, 3, 4]).into_column(),
            Series::new("city".into(), &["NYC", "LA", "SF"]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_cross_join() {
        let output = run(CommandContext::default(), Input {
            left: left_df_ipc(),
            right: right_df_ipc(),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 9); // 3 * 3 = 9 cartesian product
    }
}
