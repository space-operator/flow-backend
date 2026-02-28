use crate::prelude::*;
use super::helper::{check_response, reflect_post};

pub const NAME: &str = "reflect_get_mint_burn_quote";
const DEFINITION: &str = flow_lib::node_definition!("reflect/get_mint_burn_quote.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub quote_type: String,
    pub stablecoin_index: i64,
    pub deposit_amount: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/stablecoin/quote/{}", input.quote_type);
    let mut req = reflect_post(&ctx, &path);
    let mut body = serde_json::Map::new();
    body.insert("stablecoinIndex".into(), serde_json::json!(input.stablecoin_index));
    body.insert("depositAmount".into(), serde_json::json!(input.deposit_amount));
    req = req.json(&serde_json::Value::Object(body));
    let result = check_response(req.send().await?).await?;
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
