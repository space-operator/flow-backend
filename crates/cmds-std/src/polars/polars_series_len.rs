use crate::polars::types::series_from_ipc;
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_series_len";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_len.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub series: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub length: u64,
    pub null_count: u64,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let s = series_from_ipc(&input.series)?;
    Ok(Output {
        length: s.len() as u64,
        null_count: s.null_count() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::series_to_ipc;
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_series_ipc(name: &str, values: &[i64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_len() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[10, 20, 30, 40, 50]),
        }).await.unwrap();
        assert_eq!(output.length, 5);
        assert_eq!(output.null_count, 0);
    }
}
