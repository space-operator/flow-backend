use crate::prelude::*;
use super::{portal_post, check_response};

const NAME: &str = "swig_create_wallet_api";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_create_wallet_api.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub policy_id: String,
    #[serde(default = "default_network")]
    pub network: String,
    pub paymaster_pubkey: String,
    #[serde(default)]
    pub swig_id: Option<String>,
    #[serde(default)]
    pub wallet_address: Option<String>,
    #[serde(default)]
    pub wallet_type: Option<String>,
}

fn default_network() -> String { "mainnet".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut body = serde_json::json!({
        "policyId": input.policy_id,
        "network": input.network,
        "paymasterPubkey": input.paymaster_pubkey,
    });
    if let Some(id) = &input.swig_id {
        body["swigId"] = serde_json::json!(id);
    }
    if let Some(addr) = &input.wallet_address {
        body["walletAddress"] = serde_json::json!(addr);
    }
    if let Some(wtype) = &input.wallet_type {
        body["walletType"] = serde_json::json!(wtype);
    }

    let result = check_response(
        portal_post(&ctx, "/wallet/create", &input.api_key)
            .json(&body)
            .send()
            .await?,
    )
    .await?;
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
