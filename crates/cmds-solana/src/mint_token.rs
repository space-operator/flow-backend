use crate::{prelude::*, utils::ui_amount_to_amount};
use spl_token::instruction::mint_to_checked;

const SOLANA_MINT_TOKEN: &str = "mint_token";

const DEFINITION: &str = include_str!("../../../node-definitions/solana/mint_token.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_MINT_TOKEN)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(SOLANA_MINT_TOKEN, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::keypair")]
    mint_authority: Keypair,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    recipient: Pubkey,
    #[serde(with = "value::decimal")]
    amount: Decimal,
    decimals: Option<u8>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn get_decimals(client: &RpcClient, token_account: Pubkey) -> crate::Result<u8> {
    let source_account = client
        .get_token_account(&token_account)
        .await
        .map_err(|_| crate::Error::NotTokenAccount(token_account))?
        .ok_or(crate::Error::NotTokenAccount(token_account))?;
    Ok(source_account.token_amount.decimals)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let decimals = match input.decimals {
        Some(d) => d,
        None => get_decimals(&ctx.solana_client, input.recipient).await?,
    };
    let amount = ui_amount_to_amount(input.amount, decimals)?;

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.mint_authority.clone_keypair(),
        ]
        .into(),
        instructions: [mint_to_checked(
            &spl_token::id(),
            &input.mint_account,
            &input.recipient,
            &input.mint_authority.pubkey(),
            &[&input.fee_payer.pubkey(), &input.mint_authority.pubkey()],
            amount,
            decimals,
        )?]
        .into(),
        minimum_balance_for_rent_exemption: 0,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
