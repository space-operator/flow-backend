use bytes::Bytes;
use flow_lib::command::CommandError;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/// Maximum image download size (50 MB).
const MAX_DOWNLOAD_SIZE: usize = 50 * 1024 * 1024;

/// An image source that is either raw bytes or a URL to fetch from.
///
/// When used in a node `Input` struct, the field will accept **both**
/// `bytes` and `string` values. If a string is provided it is treated as
/// a URL and fetched with the context's HTTP client on [`resolve`][ImageInput::resolve].
#[derive(Debug, Clone)]
pub enum ImageInput {
    Bytes(Bytes),
    Url(String),
}

impl ImageInput {
    /// Resolve the image input to raw bytes.
    ///
    /// * `Bytes` variant – returned as-is.
    /// * `Url` variant – fetched via the provided `reqwest::Client`.
    pub async fn resolve(self, http: &reqwest::Client) -> Result<Bytes, CommandError> {
        match self {
            Self::Bytes(b) => Ok(b),
            Self::Url(url) => {
                let resp = http
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| CommandError::msg(format!("failed to fetch image from URL: {e}")))?
                    .error_for_status()
                    .map_err(|e| CommandError::msg(format!("image URL returned error: {e}")))?;

                // Check content-length header if available.
                if let Some(len) = resp.content_length() {
                    if len as usize > MAX_DOWNLOAD_SIZE {
                        return Err(CommandError::msg(format!(
                            "image too large: {} bytes (max {})",
                            len, MAX_DOWNLOAD_SIZE
                        )));
                    }
                }

                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| CommandError::msg(format!("failed to download image: {e}")))?;

                if bytes.len() > MAX_DOWNLOAD_SIZE {
                    return Err(CommandError::msg(format!(
                        "image too large: {} bytes (max {})",
                        bytes.len(),
                        MAX_DOWNLOAD_SIZE
                    )));
                }

                Ok(bytes)
            }
        }
    }
}

// Custom deserialization: try bytes first, fall back to string (URL).
impl<'de> Deserialize<'de> for ImageInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // value::Value can be Bytes or String; try to figure out which one we got.
        let value = value::Value::deserialize(deserializer)?;
        match value {
            value::Value::String(s) => Ok(ImageInput::Url(s)),
            other => {
                // Attempt to interpret as bytes.
                let bytes: Bytes = value::from_value(other)
                    .map_err(|e| de::Error::custom(format!("expected bytes or URL string: {e}")))?;
                Ok(ImageInput::Bytes(bytes))
            }
        }
    }
}

impl Serialize for ImageInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Bytes(b) => b.serialize(serializer),
            Self::Url(s) => s.serialize(serializer),
        }
    }
}

impl From<Bytes> for ImageInput {
    fn from(b: Bytes) -> Self {
        Self::Bytes(b)
    }
}

impl From<String> for ImageInput {
    fn from(s: String) -> Self {
        Self::Url(s)
    }
}
