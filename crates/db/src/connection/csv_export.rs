use crate::Error;

pub fn reader() -> csv::ReaderBuilder {
    let mut r = csv::ReaderBuilder::new();
    r.delimiter(b';').quote(b'\'');
    r
}

pub fn writer() -> csv::WriterBuilder {
    let mut w = csv::WriterBuilder::new();
    w.delimiter(b';').quote(b'\'');
    w
}

pub fn clear_column(data: String, column: &str) -> crate::Result<String> {
    let mut reader = reader().from_reader(data.as_bytes());
    let headers = reader
        .headers()
        .map_err(Error::parsing("csv headers"))?
        .clone();
    let col_idx = headers
        .iter()
        .position(|col| col == column)
        .ok_or_else(|| Error::not_found("column", column))?;
    let records = reader
        .records()
        .map(|r| {
            r.map_err(Error::parsing("csv iter")).map(|r| {
                r.into_iter()
                    .enumerate()
                    .map(|(i, v)| if i == col_idx { "" } else { v })
                    .collect::<csv::StringRecord>()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut buffer = Vec::new();
    let mut writer = writer().from_writer(&mut buffer);
    writer
        .write_record(&headers)
        .map_err(Error::parsing("build csv"))?;
    for r in records {
        writer
            .write_record(&r)
            .map_err(Error::parsing("build csv"))?;
    }
    writer.flush().map_err(Error::parsing("build csv"))?;
    drop(writer);
    String::from_utf8(buffer).map_err(Error::parsing("UTF8"))
}

pub fn remove_column(data: String, column: &str) -> crate::Result<String> {
    let mut reader = reader().from_reader(data.as_bytes());
    let headers = reader
        .headers()
        .map_err(Error::parsing("csv headers"))?
        .clone();
    let col_idx = headers
        .iter()
        .position(|col| col == column)
        .ok_or_else(|| Error::not_found("column", column))?;
    let records = reader
        .records()
        .map(|r| {
            r.map_err(Error::parsing("csv iter")).map(|r| {
                r.into_iter()
                    .enumerate()
                    .filter_map(|(i, v)| (i != col_idx).then_some(v))
                    .collect::<csv::StringRecord>()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut buffer = Vec::new();
    let mut writer = writer().from_writer(&mut buffer);
    writer
        .write_record(
            &headers
                .into_iter()
                .enumerate()
                .filter_map(|(i, v)| (i != col_idx).then_some(v))
                .collect::<csv::StringRecord>(),
        )
        .map_err(Error::parsing("build csv"))?;
    for r in records {
        writer
            .write_record(&r)
            .map_err(Error::parsing("build csv"))?;
    }
    writer.flush().map_err(Error::parsing("build csv"))?;
    drop(writer);

    String::from_utf8(buffer).map_err(Error::parsing("UTF8"))
}
