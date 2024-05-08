use crate::supabase_error;
use flow_lib::command::prelude::*;
use reqwest::header::{HeaderName, AUTHORIZATION};
use std::{collections::HashMap, str::FromStr};

const NAME: &str = "postgrest_execute_query";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Query,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

async fn run(mut ctx: Context, input: Input) -> Result<ValueSet, CommandError> {
    let contain_auth_header = !input.headers.iter().any(|(k, _)| {
        HeaderName::from_str(k)
            .ok()
            .map(|name| name == AUTHORIZATION)
            .unwrap_or(false)
    });
    let is_supabase = input
        .query
        .url
        .starts_with(&format!("{}/rest/v1", ctx.endpoints.supabase));

    let mut req = postgrest::Builder::from_query(input.query, ctx.http.clone()).build();
    for (k, v) in input.headers {
        req = req.header(k, v);
    }
    if contain_auth_header && is_supabase {
        tracing::info!("using JWT of user: {}", ctx.flow_owner.id);
        req = req.header("apikey", &ctx.endpoints.supabase_anon_key);
        req = req.header(AUTHORIZATION, ctx.get_jwt_header().await?);
    }
    let resp = ctx.http.execute(req.build()?).await?;

    if resp.status().is_success() {
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_lowercase(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect::<HashMap<String, String>>();

        let content_type = headers
            .get("content-type")
            .map(String::as_str)
            .unwrap_or("text/plain");
        let body: Value = if content_type.starts_with("text/") {
            resp.text().await?.into()
        } else if content_type.contains("json") {
            resp.json::<serde_json::Value>().await?.into()
        } else {
            resp.bytes().await?.into()
        };

        let headers = headers
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect::<value::Map>();

        Ok(value::map! {
            "result" => body,
            "headers" => headers,
        })
    } else {
        Err(supabase_error(resp.status(), resp).await)
    }
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/execute_query.json"))?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true })
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
