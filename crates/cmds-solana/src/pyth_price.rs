use chrono::Utc;
use flow_lib::{command::prelude::*, solana::Pubkey};
use pyth_sdk_solana::state::SolanaPriceAccount;

const NAME: &str = "pyth_price";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("pyth_price.json"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(with = "value::pubkey")]
    price_feed_id: Pubkey,
}

#[derive(Serialize, Debug)]
struct Output {
    price: i64,
}

async fn run(ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let mut sol_price_account = ctx
        .solana_client()
        .get_account(&input.price_feed_id)
        .await
        .map_err(|_| CommandError::msg("Failed to get price feed account"))?;

    let sol_price_feed =
        SolanaPriceAccount::account_to_feed(&input.price_feed_id, &mut sol_price_account)
            .map_err(|_| CommandError::msg("Invalid price feed account"))?;

    let price = sol_price_feed.get_price_unchecked();
    let age = Utc::now().timestamp() - price.publish_time;
    tracing::info!("price: {:?}, age: {}s", price, age);
    Ok(Output { price: price.price })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
