use crate::{prelude::*, utils::sol_to_lamports};

const NAME: &str = "transfer_sol";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("system_program/transfer_sol.jsonc");
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

    let instruction = solana_system_interface::instruction::transfer(
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
    use crate::test_utils;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_transfer_sol() {
        tracing_subscriber::fmt::try_init().ok();

        let sender = test_utils::test_wallet();
        let recipient = Keypair::new().pubkey();
        let ctx = test_utils::test_context();

        test_utils::ensure_funded(ctx.solana_client(), &sender.pubkey(), 0.1).await;

        let output = run(
            ctx,
            Input {
                fee_payer: None,
                sender,
                recipient,
                amount: rust_decimal_macros::dec!(0.001),
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(&output.signature);
        assert!(
            output.signature.is_some(),
            "expected a transaction signature"
        );
    }
}
