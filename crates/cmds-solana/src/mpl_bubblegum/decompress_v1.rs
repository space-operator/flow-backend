use super::MetadataBubblegum;
use crate::prelude::*;
use mpl_bubblegum::instructions::DecompressV1Builder;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account_interface::address::get_associated_token_address;

const NAME: &str = "decompress_v1_cNFT";

const DEFINITION: &str = flow_lib::node_definition!("mpl_bubblegum/decompress_v1.jsonc");

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
    pub payer: Wallet,
    pub leaf_owner: Wallet,
    #[serde(with = "value::pubkey")]
    pub voucher: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint_authority: Pubkey,
    pub metadata: MetadataBubblegum,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let token_account = get_associated_token_address(
        &input.leaf_owner.pubkey(),
        &input.mint,
    );

    let metadata_account = mpl_token_metadata::accounts::Metadata::find_pda(&input.mint).0;
    let master_edition = mpl_token_metadata::accounts::MasterEdition::find_pda(&input.mint).0;

    let ix = DecompressV1Builder::new()
        .voucher(input.voucher)
        .leaf_owner(input.leaf_owner.pubkey())
        .token_account(token_account)
        .mint(input.mint)
        .mint_authority(input.mint_authority)
        .metadata_account(metadata_account)
        .master_edition(master_edition)
        .metadata(input.metadata.into())
        .instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.leaf_owner].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
