use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum Method {
    GET,
    POST,
    DELETE,
    HEAD,
    PATCH,
    PUT,
}

#[derive(Serialize, Deserialize)]
pub struct RequestData {
    pub url: String,
    pub headers: Vec<String>,
    pub queries: Vec<String>,
    pub method: Method,
}
