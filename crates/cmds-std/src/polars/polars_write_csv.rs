use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_write_csv";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_write_csv.jsonc");

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
    pub has_header: bool,
    #[serde(default = "default_comma")]
    pub separator: String,
}

fn default_true() -> bool { true }
fn default_comma() -> String { ",".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub csv_string: String,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_ipc(&input.dataframe)?;
    let sep = input.separator.as_bytes().first().copied().unwrap_or(b',');
    let mut buf = Vec::new();
    CsvWriter::new(&mut buf)
        .include_header(input.has_header)
        .with_separator(sep)
        .finish(&mut df)
        .map_err(|e| CommandError::msg(format!("CSV write error: {e}")))?;
    let csv_string = String::from_utf8(buf)
        .map_err(|e| CommandError::msg(format!("UTF-8 error: {e}")))?;
    Ok(Output { csv_string })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;

    #[test]
    fn test_build() { build().unwrap(); }

    #[tokio::test]
    async fn test_run_write_csv() {
        let mut df = DataFrame::new(vec![
            Column::new("name".into(), &["Alice", "Bob"]),
            Column::new("age".into(), &[30i64, 25]),
        ])
        .unwrap();
        let ipc = df_to_ipc(&mut df).unwrap();

        let output = run(
            CommandContext::default(),
            Input {
                dataframe: ipc,
                has_header: true,
                separator: ",".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(output.csv_string.contains("name,age"));
        assert!(output.csv_string.contains("Alice,30"));
        assert!(output.csv_string.contains("Bob,25"));
    }

    #[tokio::test]
    async fn test_run_write_csv_no_header() {
        let mut df = DataFrame::new(vec![
            Column::new("x".into(), &[1i64, 2]),
        ])
        .unwrap();
        let ipc = df_to_ipc(&mut df).unwrap();

        let output = run(
            CommandContext::default(),
            Input {
                dataframe: ipc,
                has_header: false,
                separator: ",".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(!output.csv_string.contains("x\n"));
        assert!(output.csv_string.contains('1'));
        assert!(output.csv_string.contains('2'));
    }
}
