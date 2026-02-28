use crate::prelude::*;

pub const BASE_URL: &str = "https://api.bridge.xyz";

pub fn bridge_get(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .get(format!("{BASE_URL}{path}"))
        .header("Api-Key", api_key)
}

pub fn bridge_post(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .post(format!("{BASE_URL}{path}"))
        .header("Api-Key", api_key)
        .header("Idempotency-Key", uuid::Uuid::new_v4().to_string())
}

pub fn bridge_put(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .put(format!("{BASE_URL}{path}"))
        .header("Api-Key", api_key)
}

pub fn bridge_patch(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .patch(format!("{BASE_URL}{path}"))
        .header("Api-Key", api_key)
}

pub fn bridge_delete(ctx: &CommandContext, path: &str, api_key: &str) -> reqwest::RequestBuilder {
    ctx.http()
        .delete(format!("{BASE_URL}{path}"))
        .header("Api-Key", api_key)
}

pub async fn check_response(resp: reqwest::Response) -> Result<JsonValue, CommandError> {
    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Bridge API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    Ok(resp.json().await?)
}
