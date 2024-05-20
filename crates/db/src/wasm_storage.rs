use bytes::Bytes;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    Url,
};
use thiserror::Error as ThisError;

#[derive(Clone)]
pub struct WasmStorage {
    client: reqwest::Client,
    base_url: Url,
}

#[derive(ThisError, Debug)]
pub enum StorageError {
    #[error("invalid anon key")]
    InvalidAnonKey,
    #[error("failed to build URL: {0}")]
    BuildUrl(url::ParseError),
    #[error("failed to build URL: {0}")]
    BuildClient(reqwest::Error),
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error("{:?} {}", code, body)]
    Api {
        code: reqwest::StatusCode,
        body: String,
    },
}

impl WasmStorage {
    pub fn new(
        supabase_endpoint: Url,
        anon_key: &str,
        wasm_bucket: &str,
    ) -> Result<Self, StorageError> {
        let anon_key = HeaderValue::from_str(&format!("Bearer {}", anon_key))
            .map_err(|_| StorageError::InvalidAnonKey)?;
        let client = reqwest::Client::builder()
            .default_headers([(AUTHORIZATION, anon_key)].into_iter().collect())
            .build()
            .map_err(StorageError::BuildClient)?;
        let base_url = supabase_endpoint
            .join(&format!("storage/v1/object/{}/", wasm_bucket))
            .map_err(StorageError::BuildUrl)?;

        Ok(Self { client, base_url })
    }

    pub async fn download(&self, path: &str) -> Result<Bytes, StorageError> {
        let url = self.base_url.join(path).map_err(StorageError::BuildUrl)?;
        let resp = self.client.get(url).send().await?;
        if resp.status() == reqwest::StatusCode::OK {
            Ok(resp.bytes().await?)
        } else {
            Err(StorageError::Api {
                code: resp.status(),
                body: String::from_utf8(resp.bytes().await?.into())
                    .unwrap_or_else(|_| "<binary response body>".to_owned()),
            })
        }
    }
}
