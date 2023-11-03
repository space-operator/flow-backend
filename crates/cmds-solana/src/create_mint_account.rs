use crate::prelude::*;
use solana_sdk::program_pack::Pack;
use solana_sdk::system_instruction;
use spl_token::state::Mint;

const SOLANA_CREATE_MINT_ACCOUNT: &str = "create_mint_account";

const DEFINITION: &str = include_str!("../../../node-definitions/solana/create_mint_account.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_CREATE_MINT_ACCOUNT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(SOLANA_CREATE_MINT_ACCOUNT, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    decimals: u8,
    #[serde(with = "value::keypair")]
    mint_authority: Keypair,
    #[serde(default, with = "value::pubkey::opt")]
    freeze_authority: Option<Pubkey>,
    #[serde(with = "value::keypair")]
    mint_account: Keypair,
    #[serde(default)]
    memo: String,
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
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.mint_authority.clone_keypair(),
            input.mint_account.clone_keypair(),
        ]
        .into(),
        instructions: [
            system_instruction::create_account(
                &input.fee_payer.pubkey(),
                &input.mint_account.pubkey(),
                minimum_balance_for_rent_exemption,
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint2(
                &spl_token::id(),
                &input.mint_account.pubkey(),
                &input.mint_authority.pubkey(),
                input.freeze_authority.as_ref(),
                input.decimals,
            )?,
            spl_memo::build_memo(input.memo.as_bytes(), &[&input.fee_payer.pubkey()]),
        ]
        .into(),
        minimum_balance_for_rent_exemption,
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
