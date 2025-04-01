use flow_lib::command::prelude::*;

use serde::{Deserialize, Serialize};

use clickhouse::{error::Result, Client};
use tracing::info;
use uuid::Uuid;

use super::ClickhouseConfig;

pub const NAME: &str = "clickhouse_query";

const DEFINITION: &str = flow_lib::node_definition!("/clickhouse/clickhouse_query.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    query: String,
    clickhouse_config: ClickhouseConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    response: Vec<String>,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    info!("{:#?}", ctx.environment);

    // info!("{:#?}", ctx.user.id);
    let clickhouse_url = input.clickhouse_config.url;
    let clickhouse_user = input.clickhouse_config.user;
    let clickhouse_password = input.clickhouse_config.password;

    let query_id = Uuid::new_v4().to_string();

    let client = Client::default()
        .with_url(clickhouse_url)
        .with_user(clickhouse_user)
        .with_password(clickhouse_password)
        // Enable async insert
        .with_option("async_insert", "1")
        // Set to 1 to wait for the insert to complete
        .with_option("wait_for_async_insert", "1");

    let response = client
        .query(&input.query)
        .with_option("query_id", &query_id)
        .fetch_all::<String>()
        .await?;

    Ok(Output { response })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_clickhouse_config_from_env() -> ClickhouseConfig {
        dotenvy::dotenv().ok();

        ClickhouseConfig {
            url: std::env::var("clickhouse_url")
                .unwrap_or_else(|_| "http://localhost:8123".to_string()),
            user: std::env::var("clickhouse_user").unwrap_or_else(|_| "default".to_string()),
            password: std::env::var("clickhouse_password").unwrap_or_default(),
            database: None,
        }
    }

    #[tokio::test]
    async fn test_json_extract() {
        let clickhouse_config = get_clickhouse_config_from_env();

        // Convert struct to serialized form
        let config_value = serde_json::to_value(&clickhouse_config).unwrap_or_default();

        let inputs = value::map! {
            "clickhouse_config" => config_value,
            "query" => "SHOW USERS",
        };

        let outputs = build().unwrap().run(<_>::default(), inputs).await.unwrap();
        dbg!(&outputs.get("response"));
    }
}
