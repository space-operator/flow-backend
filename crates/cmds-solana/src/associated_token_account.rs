use crate::prelude::*;
use solana_program::program_pack::Pack;
use spl_associated_token_account::instruction::create_associated_token_account;

const SOLANA_ASSOCIATED_TOKEN_ACCOUNT: &str = "associated_token_account";

const DEFINITION: &str =
    include_str!("../../../node-definitions/solana/associated_token_account.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_ASSOCIATED_TOKEN_ACCOUNT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(
    SOLANA_ASSOCIATED_TOKEN_ACCOUNT,
    |_| { build() }
));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    owner: Pubkey,
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .await?;

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
            signers: [input.fee_payer.clone_keypair()].into(),
            minimum_balance_for_rent_exemption,
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
