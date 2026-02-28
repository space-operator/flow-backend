use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flow_lib::command::prelude::*;
use polars::prelude::*;
use std::io::Cursor;

// ── DataFrame IPC serialization ─────────────────────────────────────────

pub fn df_to_ipc(df: &mut DataFrame) -> Result<String, CommandError> {
    let mut buf = Vec::new();
    IpcWriter::new(&mut buf)
        .with_compression(Some(IpcCompression::ZSTD))
        .finish(df)
        .map_err(|e| CommandError::msg(format!("IPC write error: {e}")))?;
    Ok(BASE64.encode(&buf))
}

pub fn df_from_ipc(base64_str: &str) -> Result<DataFrame, CommandError> {
    let bytes = BASE64
        .decode(base64_str)
        .map_err(|e| CommandError::msg(format!("Base64 decode error: {e}")))?;
    let cursor = Cursor::new(bytes);
    IpcReader::new(cursor)
        .finish()
        .map_err(|e| CommandError::msg(format!("IPC read error: {e}")))
}

// ── DataFrame JSON (for viewing) ────────────────────────────────────────

pub fn df_to_json(df: &mut DataFrame) -> Result<JsonValue, CommandError> {
    let mut buf = Vec::new();
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::Json)
        .finish(df)
        .map_err(|e| CommandError::msg(format!("JSON write error: {e}")))?;
    let json_str = String::from_utf8(buf)
        .map_err(|e| CommandError::msg(format!("UTF-8 error: {e}")))?;
    serde_json::from_str(&json_str)
        .map_err(|e| CommandError::msg(format!("JSON parse error: {e}")))
}

// ── Series IPC serialization ────────────────────────────────────────────

pub fn series_to_ipc(series: &Series) -> Result<String, CommandError> {
    let mut df = DataFrame::new(vec![series.clone().into_column()])
        .map_err(|e| CommandError::msg(format!("Series to DataFrame error: {e}")))?;
    df_to_ipc(&mut df)
}

pub fn series_from_ipc(base64_str: &str) -> Result<Series, CommandError> {
    let df = df_from_ipc(base64_str)?;
    let cols = df.get_columns();
    if cols.is_empty() {
        return Err(CommandError::msg("Empty DataFrame, expected one column for Series"));
    }
    Ok(cols[0].as_materialized_series().clone())
}

pub fn series_to_json(series: &Series) -> Result<JsonValue, CommandError> {
    let mut df = DataFrame::new(vec![series.clone().into_column()])
        .map_err(|e| CommandError::msg(format!("Series to DataFrame error: {e}")))?;
    df_to_json(&mut df)
}

// ── DataFrame from JSON input (for create nodes) ───────────────────────

pub fn df_from_json_value(value: &JsonValue) -> Result<DataFrame, CommandError> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| CommandError::msg(format!("JSON serialize error: {e}")))?;
    let cursor = Cursor::new(json_str.as_bytes());
    JsonReader::new(cursor)
        .finish()
        .map_err(|e| CommandError::msg(format!("JSON to DataFrame error: {e}")))
}

// ── Helper: build dual output (IPC + JSON) ──────────────────────────────

pub fn dual_output(df: &mut DataFrame) -> Result<(String, JsonValue), CommandError> {
    let ipc = df_to_ipc(df)?;
    let json = df_to_json(df)?;
    Ok((ipc, json))
}

pub fn dual_series_output(series: &Series) -> Result<(String, JsonValue), CommandError> {
    let ipc = series_to_ipc(series)?;
    let json = series_to_json(series)?;
    Ok((ipc, json))
}

// ── Parse dtype string to Polars DataType ───────────────────────────────

pub fn parse_dtype(dtype_str: &str) -> Result<DataType, CommandError> {
    match dtype_str.to_lowercase().as_str() {
        "bool" | "boolean" => Ok(DataType::Boolean),
        "u8" | "uint8" => Ok(DataType::UInt8),
        "u16" | "uint16" => Ok(DataType::UInt16),
        "u32" | "uint32" => Ok(DataType::UInt32),
        "u64" | "uint64" => Ok(DataType::UInt64),
        "i8" | "int8" => Ok(DataType::Int8),
        "i16" | "int16" => Ok(DataType::Int16),
        "i32" | "int32" => Ok(DataType::Int32),
        "i64" | "int64" => Ok(DataType::Int64),
        "f32" | "float32" => Ok(DataType::Float32),
        "f64" | "float64" => Ok(DataType::Float64),
        "str" | "string" | "utf8" => Ok(DataType::String),
        "date" => Ok(DataType::Date),
        "datetime" => Ok(DataType::Datetime(TimeUnit::Microseconds, None)),
        "duration" => Ok(DataType::Duration(TimeUnit::Microseconds)),
        "time" => Ok(DataType::Time),
        "binary" => Ok(DataType::Binary),
        "null" => Ok(DataType::Null),
        other => Err(CommandError::msg(format!("Unknown dtype: {other}"))),
    }
}

// ── Parse column name list from JSON ────────────────────────────────────

pub fn parse_column_names(value: &JsonValue) -> Result<Vec<String>, CommandError> {
    match value {
        JsonValue::Array(arr) => arr
            .iter()
            .map(|v| match v {
                JsonValue::String(s) => Ok(s.clone()),
                other => Err(CommandError::msg(format!(
                    "Expected string in column list, got: {other}"
                ))),
            })
            .collect(),
        JsonValue::String(s) => Ok(vec![s.clone()]),
        other => Err(CommandError::msg(format!(
            "Expected array or string for column names, got: {other}"
        ))),
    }
}
