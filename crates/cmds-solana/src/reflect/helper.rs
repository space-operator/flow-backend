use crate::prelude::*;

pub const BASE_URL: &str = "https://prod.api.reflect.money";

/// Public GET request (no authentication required).
/// Most Reflect endpoints are permissionless.
pub fn reflect_get(ctx: &CommandContext, path: &str) -> reqwest::RequestBuilder {
    ctx.http().get(format!("{BASE_URL}{path}"))
}

/// Public POST request (no authentication required).
pub fn reflect_post(ctx: &CommandContext, path: &str) -> reqwest::RequestBuilder {
    ctx.http().post(format!("{BASE_URL}{path}"))
}

/// Authenticated GET request with Reflect-API-Key header.
/// Used by: get_integration_stats, get_integration_historical_stats.
pub fn reflect_get_auth(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .get(format!("{BASE_URL}{path}"))
        .header("Reflect-API-Key", api_key)
}

/// Authenticated POST request with Reflect-API-Key header.
/// Used by: whitelist_address.
pub fn reflect_post_auth(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .post(format!("{BASE_URL}{path}"))
        .header("Reflect-API-Key", api_key)
}

pub async fn check_response(resp: reqwest::Response) -> Result<JsonValue, CommandError> {
    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Reflect API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    Ok(resp.json().await?)
}
