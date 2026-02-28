use crate::prelude::*;
use flow_lib::solana::Pubkey;

use super::helper::{parse_pull_feed_result, PRECISION};

pub const NAME: &str = "switchboard_get_price";
const DEFINITION: &str = flow_lib::node_definition!("switchboard/switchboard_get_price.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(with = "value::pubkey")]
    feed_account: Pubkey,
    #[serde(default)]
    max_staleness_slots: Option<u64>,
}

#[derive(Serialize, Debug)]
struct Output {
    price: f64,
    std_dev: f64,
    mean: f64,
    min_value: f64,
    max_value: f64,
    num_samples: u8,
    slot: u64,
    result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let account = ctx
        .solana_client()
        .get_account(&input.feed_account)
        .await
        .map_err(|_| CommandError::msg("Failed to fetch Switchboard feed account"))?;

    let result = parse_pull_feed_result(&account.data)?;

    if let Some(max_staleness) = input.max_staleness_slots {
        let clock = ctx
            .solana_client()
            .get_slot()
            .await
            .map_err(|e| CommandError::msg(format!("Failed to get current slot: {e}")))?;
        let age = clock.saturating_sub(result.slot);
        if age > max_staleness {
            return Err(CommandError::msg(format!(
                "Feed data is stale: {age} slots old (max allowed: {max_staleness})"
            )));
        }
    }

    let to_f64 = |v: i128| -> f64 { v as f64 / PRECISION as f64 };

    let result_json = serde_json::json!({
        "value": to_f64(result.value),
        "std_dev": to_f64(result.std_dev),
        "mean": to_f64(result.mean),
        "range": to_f64(result.range),
        "min_value": to_f64(result.min_value),
        "max_value": to_f64(result.max_value),
        "num_samples": result.num_samples,
        "slot": result.slot,
        "min_slot": result.min_slot,
        "max_slot": result.max_slot,
    });

    Ok(Output {
        price: to_f64(result.value),
        std_dev: to_f64(result.std_dev),
        mean: to_f64(result.mean),
        min_value: to_f64(result.min_value),
        max_value: to_f64(result.max_value),
        num_samples: result.num_samples,
        slot: result.slot,
        result: result_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
