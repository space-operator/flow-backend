use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_shape";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_shape.jsonc");

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
    pub rows: u64,
    pub columns: u64,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    Ok(Output {
        rows: df.height() as u64,
        columns: df.width() as u64,
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
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), Some("Charlie"), Some("Alice")]).into_column(),
            Series::new("age".into(), &[Some(30i64), Some(25), Some(35), Some(30)]).into_column(),
            Series::new("score".into(), &[Some(88.5f64), Some(92.0), Some(75.3), Some(91.0)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
        }).await.unwrap();
        assert_eq!(output.rows, 4);
        assert_eq!(output.columns, 3);
    }
}
