use crate::prelude::*;

pub const BASE_URL: &str = "https://payment.ardrive.io/v1";

/// Optional ArDrive wallet authentication headers.
pub struct ArDriveAuth {
    pub x_signature: Option<String>,
    pub x_nonce: Option<String>,
    pub x_public_key: Option<String>,
}

/// Build a GET request to the ArDrive Turbo payment API.
pub fn ardrive_get(ctx: &CommandContext, path: &str) -> reqwest::RequestBuilder {
    ctx.http().get(format!("{BASE_URL}{path}"))
}

/// Build a POST request to the ArDrive Turbo payment API.
pub fn ardrive_post(ctx: &CommandContext, path: &str) -> reqwest::RequestBuilder {
    ctx.http().post(format!("{BASE_URL}{path}"))
}

/// Conditionally attach signature auth headers to a request builder.
pub fn apply_auth(
    mut builder: reqwest::RequestBuilder,
    auth: &ArDriveAuth,
) -> reqwest::RequestBuilder {
    if let Some(ref sig) = auth.x_signature {
        builder = builder.header("x-signature", sig);
    }
    if let Some(ref nonce) = auth.x_nonce {
        builder = builder.header("x-nonce", nonce);
    }
    if let Some(ref pk) = auth.x_public_key {
        builder = builder.header("x-public-key", pk);
    }
    builder
}

/// Standard error handling for ArDrive API responses.
pub async fn check_response(resp: reqwest::Response) -> Result<JsonValue, CommandError> {
    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "ArDrive API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    Ok(resp.json().await?)
}
