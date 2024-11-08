use crate::prelude::*;
use spl_associated_token_account::instruction::create_associated_token_account;

const SOLANA_ASSOCIATED_TOKEN_ACCOUNT: &str = "associated_token_account";

const DEFINITION: &str = flow_lib::node_definition!("associated_token_account.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_ASSOCIATED_TOKEN_ACCOUNT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(
    SOLANA_ASSOCIATED_TOKEN_ACCOUNT,
    |_| { build() }
));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsPubkey")]
    owner: Pubkey,
    fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    mint_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let instruction = create_associated_token_account(
        &input.fee_payer.pubkey(),
        &input.owner,
        &input.mint_account,
        &spl_token::id(),
    );

    let associated_token_account = instruction.accounts[1].pubkey;

    let instructions = if input.submit {
        Instructions {
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer].into(),
            instructions: [instruction].into(),
        }
    } else {
        <_>::default()
    };

    let signature = ctx
        .execute(
            instructions,
            value::map! {
                "associated_token_account" => associated_token_account,
            },
        )
        .await?
        .signature;

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
