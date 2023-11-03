use crate::prelude::*;
use solana_program::program_pack::Pack;
use solana_program::{system_instruction, system_program};

const SOLANA_CREATE_TOKEN_ACCOUNT: &str = "create_token_account";

const DEFINITION: &str = include_str!("../../../node-definitions/solana/create_token_account.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_CREATE_TOKEN_ACCOUNT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(SOLANA_CREATE_TOKEN_ACCOUNT, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    owner: Pubkey,
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    #[serde(with = "value::keypair")]
    token_account: Keypair,
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

    let account = input.token_account.pubkey();
    let system_account_ok = false;
    let instructions = [
        system_instruction::create_account(
            &input.fee_payer.pubkey(),
            &account,
            minimum_balance_for_rent_exemption,
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_account(
            &spl_token::id(),
            &account,
            &input.mint_account,
            &input.owner,
        )?,
    ]
    .into();

    // TODO: with bundling, this data might be outdated when tx is submitted
    if let Some(account_data) = ctx
        .solana_client
        .get_account_with_commitment(&account, ctx.solana_client.commitment())
        .await?
        .value
    {
        if !(account_data.owner == system_program::id() && system_account_ok) {
            return Err(crate::Error::custom(anyhow::anyhow!(
                "Error: Account already exists: {}",
                account
            ))
            .into());
        }
    }

    let instructions = if input.submit {
        Instructions {
            fee_payer: input.fee_payer.pubkey(),
            signers: [
                input.fee_payer.clone_keypair(),
                input.token_account.clone_keypair(),
            ]
            .into(),
            minimum_balance_for_rent_exemption,
            instructions,
        }
    } else {
        <_>::default()
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
}
