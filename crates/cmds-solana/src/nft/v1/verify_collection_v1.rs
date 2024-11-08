use crate::prelude::*;
use mpl_token_metadata::accounts::{MasterEdition, Metadata};
use solana_program::{system_program, sysvar};

// Command Name
const NAME: &str = "verify_collection_v1";

const DEFINITION: &str = flow_lib::node_definition!("nft/v1/verify_collection_v1.json");

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
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    pub fee_payer: Wallet,
    pub collection_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub collection_mint_account: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub delegate_record: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);
    let (collection_metadata, _) = Metadata::find_pda(&input.collection_mint_account);

    let (collection_master_edition, _) = MasterEdition::find_pda(&input.collection_mint_account);

    let accounts = mpl_token_metadata::instructions::VerifyCollectionV1 {
        authority: input.collection_authority.pubkey(),
        delegate_record: input.delegate_record,
        metadata: metadata_account,
        collection_mint: input.collection_mint_account,
        collection_metadata: Some(collection_metadata),
        collection_master_edition: Some(collection_master_edition),
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
    };

    let ins = accounts.instruction();

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.collection_authority].into(),
        instructions: [ins].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
