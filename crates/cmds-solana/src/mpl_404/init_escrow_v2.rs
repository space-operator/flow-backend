use crate::prelude::*;
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::InitEscrowV2Builder;

const NAME: &str = "init_escrow_v2";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/init_escrow_v2.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,

    // accounts
    authority: Wallet,

    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.authority.pubkey().as_ref()],
        &mpl_hybrid::ID,
    );

    let init_escrow_v2_ix = InitEscrowV2Builder::new()
        .escrow(escrow)
        .authority(input.authority.pubkey())
        .instruction();

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [init_escrow_v2_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { escrow, signature })
}

#[cfg(test)]
mod tests {
    use crate::utils::ui_amount_to_amount;

    use super::*;

    async fn transfer_sol(
        ctx: &Context,
        from_pubkey: &Wallet,
        to_pubkey: &Pubkey,
        amount: Decimal,
    ) -> crate::Result<Signature> {
        let ix = solana_sdk::system_instruction::transfer(
            &from_pubkey.pubkey(),
            to_pubkey,
            ui_amount_to_amount(amount, 9).unwrap(),
        );

        let (mut transfer_sol_tx, recent_blockhash) =
            execute(&ctx.solana_client, &from_pubkey.pubkey(), &[ix])
                .await
                .unwrap();

        transfer_sol_tx
            .try_sign(&[from_pubkey.keypair().unwrap()], recent_blockhash)
            .unwrap();

        submit_transaction(&ctx.solana_client, transfer_sol_tx).await
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        let fee_payer = Wallet::Keypair(Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ"));
        let authority = Wallet::Keypair(Keypair::new());

        let transfer_sol_signature = transfer_sol(
            &ctx,
            &fee_payer,
            &authority.pubkey(),
            rust_decimal_macros::dec!(0.03),
        )
        .await
        .unwrap();

        dbg!(transfer_sol_signature);

        let output = run(
            ctx,
            super::Input {
                fee_payer,
                authority,
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.escrow);
        dbg!(output.signature.unwrap());
    }
}
