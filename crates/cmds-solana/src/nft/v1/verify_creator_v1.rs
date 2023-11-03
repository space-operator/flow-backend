use crate::prelude::*;
use mpl_token_metadata::accounts::{MasterEdition, Metadata};
use solana_program::{system_program, sysvar};

// Command Name
const NAME: &str = "verify_creator_v1";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/NFT/v1/verify_creator_v1.json");

fn build() -> BuildResult {
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
    pub authority: Keypair,
    #[serde(default, with = "value::pubkey::opt")]
    pub delegate_record: Option<Pubkey>,
    #[serde(with = "value::pubkey")]
    pub metadata: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection_mint_account: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mut collection_metadata: Option<Pubkey> = None;
    let mut collection_master_edition: Option<Pubkey> = None;

    if let Some(collection_mint_account) = input.collection_mint_account {
        let (metadata, _) = Metadata::find_pda(&collection_mint_account);
        collection_metadata = Some(metadata);

        let (master_edition, _) = MasterEdition::find_pda(&collection_mint_account);
        collection_master_edition = Some(master_edition);
    }

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::accounts::MasterEdition,
        >())
        .await?;

    let accounts = mpl_token_metadata::instructions::VerifyCreatorV1 {
        authority: input.authority.pubkey(),
        delegate_record: input.delegate_record,
        metadata: input.metadata,
        collection_mint: input.collection_mint_account,
        collection_metadata: collection_metadata,
        collection_master_edition: collection_master_edition,
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
    };

    let ins = accounts.instruction();

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [ins].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
