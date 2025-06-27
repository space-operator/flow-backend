use polars::{
    error::PolarsError,
    frame::DataFrame,
    io::{SerReader, SerWriter},
    prelude::{CsvParseOptions, CsvReadOptions, CsvWriter, NullValues, PlSmallStr},
    series::Series,
};
use std::{io::Cursor, iter::repeat_n};

pub fn format_sql_columns(df: &DataFrame) -> String {
    df.get_column_names()
        .iter()
        .map(|name| format!("{name:?}"))
        .collect::<Vec<String>>()
        .join(",")
}

/// Set column to null
pub fn clear_column(df: &mut DataFrame, column: &str) -> Result<(), PolarsError> {
    df.apply(column, |c| {
        repeat_n::<Option<String>>(None, c.len()).collect::<Series>()
    })?;
    Ok(())
}

fn read_df_impl(csv: &[u8]) -> Result<DataFrame, PolarsError> {
    CsvReadOptions::default()
        .with_parse_options(
            CsvParseOptions::default()
                .with_separator(b';')
                .with_quote_char(Some(b'\''))
                // https://github.com/pola-rs/polars/issues/21239
                .with_null_values(Some(NullValues::AllColumnsSingle(PlSmallStr::from_static(
                    "''",
                )))),
        )
        .into_reader_with_file_handle(Cursor::new(csv))
        .finish()
}

pub fn read_df<T: AsRef<[u8]>>(csv: T) -> Result<DataFrame, PolarsError> {
    let bytes = csv.as_ref();
    read_df_impl(bytes)
}

pub fn write_df(df: &mut DataFrame) -> Result<Vec<u8>, PolarsError> {
    let mut buffer = Vec::<u8>::new();
    CsvWriter::new(&mut buffer)
        .include_header(true)
        .with_separator(b';')
        .with_quote_char(b'\'')
        .with_null_value("".to_owned())
        .finish(df)?;
    Ok(buffer)
}

pub mod df_serde {
    use super::{read_df, write_df};
    use polars::frame::DataFrame;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(df: &DataFrame, s: S) -> Result<S::Ok, S::Error> {
        String::from_utf8(write_df(&mut df.clone()).map_err(serde::ser::Error::custom)?)
            .map_err(serde::ser::Error::custom)?
            .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DataFrame, D::Error> {
        let text = String::deserialize(d)?;
        read_df(text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use polars::df;

    use super::*;

    #[test]
    fn test_null_value() {
        let mut df = df!(
            "foo" => [None, Some("")],
            "bar" => [Some(""), None],
        )
        .unwrap();
        let csv = String::from_utf8(write_df(&mut df).unwrap()).unwrap();
        const CSV: &str = "foo;bar
;''
'';
";
        assert_eq!(csv, CSV);
        let parsed = read_df(csv).unwrap();
        assert_eq!(df, parsed);
    }

    #[test]
    fn test_polars() {
        // https://github.com/pola-rs/polars/issues/21239
        let df = CsvReadOptions::default()
            .with_parse_options(
                CsvParseOptions::default()
                    .with_quote_char(Some(b'\''))
                    .with_null_values(Some(NullValues::AllColumnsSingle(PlSmallStr::from_static(
                        "''",
                    )))),
            )
            .into_reader_with_file_handle(Cursor::new("foo\n\n''"))
            .finish()
            .unwrap();

        assert_eq!(df, df!("foo" => [None, Some("")]).unwrap());
    }
}
