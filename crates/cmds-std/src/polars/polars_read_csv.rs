use crate::polars::types::{dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;
use std::io::Cursor;

pub const NAME: &str = "polars_read_csv";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_read_csv.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub csv_string: String,
    #[serde(default = "default_true")]
    pub has_header: bool,
    #[serde(default = "default_comma")]
    pub separator: String,
    #[serde(default = "default_infer_len")]
    pub infer_schema_length: u32,
}

fn default_true() -> bool { true }
fn default_comma() -> String { ",".to_string() }
fn default_infer_len() -> u32 { 100 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let sep = input
        .separator
        .as_bytes()
        .first()
        .copied()
        .unwrap_or(b',');
    let cursor = Cursor::new(input.csv_string.as_bytes());
    let parse_options = CsvParseOptions::default().with_separator(sep);
    let mut df = CsvReadOptions::default()
        .with_has_header(input.has_header)
        .with_infer_schema_length(Some(input.infer_schema_length as usize))
        .with_parse_options(parse_options)
        .into_reader_with_file_handle(cursor)
        .finish()
        .map_err(|e| CommandError::msg(format!("CSV parse error: {e}")))?;
    let (ipc, json) = dual_output(&mut df)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_from_ipc;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run_basic_csv() {
        let csv = "name,age\nAlice,30\nBob,25\n".to_string();
        let output = run(
            CommandContext::default(),
            Input {
                csv_string: csv,
                has_header: true,
                separator: ",".to_string(),
                infer_schema_length: 100,
            },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (2, 2));
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert_eq!(col_names, vec!["name", "age"]);
    }

    #[tokio::test]
    async fn test_run_tab_separated() {
        let tsv = "name\tage\nAlice\t30\nBob\t25\n".to_string();
        let output = run(
            CommandContext::default(),
            Input {
                csv_string: tsv,
                has_header: true,
                separator: "\t".to_string(),
                infer_schema_length: 100,
            },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (2, 2));
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert_eq!(col_names, vec!["name", "age"]);
    }

    #[tokio::test]
    async fn test_run_no_header() {
        let csv = "Alice,30\nBob,25\n".to_string();
        let output = run(
            CommandContext::default(),
            Input {
                csv_string: csv,
                has_header: false,
                separator: ",".to_string(),
                infer_schema_length: 100,
            },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (2, 2));
    }
}
