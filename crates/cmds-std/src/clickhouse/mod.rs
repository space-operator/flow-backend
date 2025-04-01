use serde::{Deserialize, Serialize};

pub mod batch_insert;
pub mod query;

#[derive(Serialize, Deserialize, Debug)]
pub struct ClickhouseConfig {
    url: String,
    user: String,
    password: String,
    database: Option<String>,
}
