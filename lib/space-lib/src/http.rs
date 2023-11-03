use crate::{common::Method, ffi, Result};

pub struct Request {
    url: String,
    method: Method,
    headers: Vec<String>,
    queries: Vec<String>,
}

impl Request {
    /// Create a new request with method.
    pub fn new<T: Into<String>>(url: T, method: Method) -> Self {
        Self {
            url: url.into(),
            method,
            headers: Vec::new(),
            queries: Vec::new(),
        }
    }

    /// Make a GET request.
    pub fn get<T: Into<String>>(url: T) -> Self {
        Self::new(url, Method::GET)
    }

    /// Make a POST request.
    pub fn post<T: Into<String>>(url: T) -> Self {
        Self::new(url, Method::POST)
    }

    /// Make a DELETE request.
    pub fn delete<T: Into<String>>(url: T) -> Self {
        Self::new(url, Method::DELETE)
    }

    /// Make a HEAD request.
    pub fn head<T: Into<String>>(url: T) -> Self {
        Self::new(url, Method::HEAD)
    }

    /// Make a PATCH request.
    pub fn patch<T: Into<String>>(url: T) -> Self {
        Self::new(url, Method::PATCH)
    }

    /// Make a PUT request.
    pub fn put<T: Into<String>>(url: T) -> Self {
        Self::new(url, Method::PUT)
    }

    /// Set a header field.
    pub fn set<T: ToString, U: ToString>(mut self, header: T, value: U) -> Self {
        self.headers.extend([header.to_string(), value.to_string()]);
        self
    }

    /// Set a query parameter.
    pub fn query<T: ToString, U: ToString>(mut self, param: T, value: U) -> Self {
        self.queries.extend([param.to_string(), value.to_string()]);
        self
    }

    /// Send the request.
    pub fn call(self) -> Result<Response> {
        let bytes = ffi::call_request(self.url, self.headers, self.queries, self.method)?;
        Ok(Response { bytes })
    }
}

pub struct Response {
    bytes: Vec<u8>,
}

impl Response {
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    pub fn into_string(self) -> Result<String> {
        Ok(std::str::from_utf8(&self.bytes).map(|it| it.to_string())?)
    }

    #[cfg(feature = "json")]
    pub fn into_json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        let json = std::str::from_utf8(&self.bytes)?;
        Ok(serde_json::from_str(json)?)
    }
}
