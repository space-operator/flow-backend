use crate::{prelude::*, utils::sol_to_lamports};

const NAME: &str = "transfer_sol";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("transfer_sol.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub sender: Keypair,
    #[serde(with = "value::pubkey")]
    pub recipient: Pubkey,
    #[serde(with = "value::decimal")]
    pub amount: Decimal,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let amount = sol_to_lamports(input.amount)?;

    let instruction =
        solana_sdk::system_instruction::transfer(&input.sender.pubkey(), &input.recipient, amount);

    let instructions = if input.submit {
        Instructions {
            fee_payer: input.sender.pubkey(),
            signers: vec![input.sender.clone_keypair()],

            instructions: [instruction].into(),
        }
    } else {
        Instructions::default()
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_valid() {
        tracing_subscriber::fmt::try_init().ok();
        let ctx = Context::default();

        let sender = Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ");
        let recipient = solana_sdk::pubkey!("GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9");

        let balance = ctx
            .solana_client
            .get_balance(&sender.pubkey())
            .await
            .unwrap() as f64
            / 1_000_000_000.0;

        if balance < 0.1 {
            let _ = ctx
                .solana_client
                .request_airdrop(&sender.pubkey(), 1_000_000_000)
                .await;
        }

        // Transfer
        let output = run(
            ctx,
            Input {
                sender,
                recipient,
                amount: rust_decimal_macros::dec!(0.1),
                submit: true,
            },
        )
        .await
        .unwrap();
        dbg!(output.signature.unwrap());
    }
}
