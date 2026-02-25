use crate::prelude::*;
use ::mpl_token_metadata::accounts::Metadata;
use ::mpl_token_metadata::instructions::SetTokenStandardBuilder;

// Command Name
const NAME: &str = "set_token_standard";

const DEFINITION: &str = flow_lib::node_definition!("mpl_token_metadata/set_token_standard.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub update_authority: Wallet,
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

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let instructions = vec![
        SetTokenStandardBuilder::new()
            .metadata(metadata_account)
            .update_authority(input.update_authority.pubkey())
            .mint(input.mint_account)
            .edition(input.edition_account)
            .instruction(),
    ];

    let ins = Instructions {
lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.update_authority].into(),
        instructions,
    };

    let signature: Option<Signature> = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
