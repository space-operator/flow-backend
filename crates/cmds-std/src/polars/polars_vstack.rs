use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_vstack";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_vstack.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub top: String,
    pub bottom: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut top_df = df_from_ipc(&input.top)?;
    let bottom_df = df_from_ipc(&input.bottom)?;

    top_df
        .vstack_mut(&bottom_df)
        .map_err(|e| CommandError::msg(format!("Vstack error: {e}")))?;

    let (ipc, json) = dual_output(&mut top_df)?;
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

    #[tokio::test]
    async fn test_run() {
        let mut top = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob"]).into_column(),
            Series::new("age".into(), &[30i64, 25]).into_column(),
        ]).unwrap();
        let mut bottom = DataFrame::new(vec![
            Series::new("name".into(), &["Charlie", "Diana", "Eve"]).into_column(),
            Series::new("age".into(), &[35i64, 28, 22]).into_column(),
        ]).unwrap();

        let output = run(CommandContext::default(), Input {
            top: df_to_ipc(&mut top).unwrap(),
            bottom: df_to_ipc(&mut bottom).unwrap(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 5, "vstack should produce 5 rows (2 + 3)");
        assert_eq!(df.width(), 2, "vstack should preserve 2 columns");
    }
}
