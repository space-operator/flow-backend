use crate::prelude::*;
use super::helper::{check_response, reflect_post};

pub const NAME: &str = "burn_stablecoin";
const DEFINITION: &str = flow_lib::node_definition!("reflect/burn_stablecoin.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(default)]
    pub cluster: Option<String>,
    pub stablecoin_index: i64,
    pub deposit_amount: f64,
    pub signer: String,
    #[serde(default)]
    pub minimum_received: Option<f64>,
    #[serde(default)]
    pub collateral_mint: Option<String>,
    #[serde(default)]
    pub fee_payer: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/stablecoin/burn";
    let mut req = reflect_post(&ctx, path);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.cluster {
        query.push(("cluster", val.as_str()));
    }
    if !query.is_empty() {
        req = req.query(&query);
    }
    let mut body = serde_json::Map::new();
    body.insert("stablecoinIndex".into(), serde_json::json!(input.stablecoin_index));
    body.insert("depositAmount".into(), serde_json::json!(input.deposit_amount));
    body.insert("signer".into(), serde_json::Value::String(input.signer.clone()));
    if let Some(ref val) = input.minimum_received {
        body.insert("minimumReceived".into(), serde_json::json!(val));
    }
    if let Some(ref val) = input.collateral_mint {
        body.insert("collateralMint".into(), serde_json::Value::String(val.clone()));
    }
    if let Some(ref val) = input.fee_payer {
        body.insert("feePayer".into(), serde_json::Value::String(val.clone()));
    }
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
