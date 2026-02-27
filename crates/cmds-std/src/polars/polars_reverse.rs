use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_reverse";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_reverse.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let mut result = df.reverse();
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
    use polars::prelude::*;

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
    async fn test_run_reverse() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let names = df.column("name").unwrap();
        assert_eq!(names.str().unwrap().get(0).unwrap(), "Charlie");
        assert_eq!(names.str().unwrap().get(1).unwrap(), "Bob");
        assert_eq!(names.str().unwrap().get(2).unwrap(), "Alice");
    }
}
