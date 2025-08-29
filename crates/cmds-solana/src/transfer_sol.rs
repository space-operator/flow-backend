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

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    pub fee_payer: Option<Wallet>,
    pub sender: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub recipient: Pubkey,
    #[serde_as(as = "AsDecimal")]
    pub amount: Decimal,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let amount = sol_to_lamports(input.amount)?;

    let instruction = solana_program::system_instruction::transfer(
        &input.sender.pubkey(),
        &input.recipient,
        amount,
    );

    let sender_pubkey = input.sender.pubkey();
    let mut signers = vec![input.sender];
    if let Some(fee_payer) = input.fee_payer
        && fee_payer.pubkey() != sender_pubkey
    {
        signers.insert(0, fee_payer);
    }
    let fee_payer = signers[0].pubkey();
    let instructions = if input.submit {
        Instructions {
            lookup_tables: None,
            fee_payer,
            signers,
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
    #[ignore]
    async fn test_valid() {
        tracing_subscriber::fmt::try_init().ok();
        let ctx = CommandContext::default();

        let sender: Wallet = Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into();
        let recipient = solana_program::pubkey!("GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9");

        let balance = ctx
            .solana_client()
            .get_balance(&sender.pubkey())
            .await
            .unwrap() as f64
            / 1_000_000_000.0;

        if balance < 0.1 {
            let _ = ctx
                .solana_client()
                .request_airdrop(&sender.pubkey(), 1_000_000_000)
                .await;
        }

        // Transfer
        let output = run(
            ctx,
            super::Input {
                fee_payer: None,
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
