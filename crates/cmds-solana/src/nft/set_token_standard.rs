use crate::prelude::*;

// Command Name
const NAME: &str = "set_token_standard";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/NFT/set_token_standard.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::keypair")]
    pub update_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub edition_account: Option<Pubkey>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&input.mint_account);

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(
            100, // std::mem::size_of::<
                // mpl_token_metadata::state::VerifyCollection,
                // >(),
        )
        .await?;

    let instructions = vec![mpl_token_metadata::instruction::set_token_standard(
        mpl_token_metadata::id(),
        metadata_account,
        input.update_authority.pubkey(),
        input.mint_account,
        input.edition_account,
    )];

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.update_authority.clone_keypair()].into(),
        instructions,
        minimum_balance_for_rent_exemption,
    };

    let signature: Option<Signature> = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
