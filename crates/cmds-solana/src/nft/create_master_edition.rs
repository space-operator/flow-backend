use crate::prelude::*;
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    instructions::CreateMasterEditionV3InstructionArgs,
};
use solana_program::system_program;

// Command Name
const NAME: &str = "create_master_edition";

const DEFINITION: &str = flow_lib::node_definition!("nft/create_master_edition.json");

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
    update_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint_authority: Pubkey,
    pub fee_payer: Wallet,
    pub max_supply: Option<u64>,
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

    let (master_edition_account, _) = MasterEdition::find_pda(&input.mint_account);

    let create_ix = mpl_token_metadata::instructions::CreateMasterEditionV3 {
        edition: master_edition_account,
        mint: input.mint_account,
        update_authority: input.update_authority.pubkey(),
        mint_authority: input.mint_authority,
        payer: input.fee_payer.pubkey(),
        metadata: metadata_account,
        token_program: spl_token::id(),
        system_program: system_program::id(),
        rent: None,
    };

    let args = CreateMasterEditionV3InstructionArgs {
        max_supply: input.max_supply,
    };

    let create_ix = create_ix.instruction(args);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.update_authority].into(),
        instructions: [create_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "metadata_account" => metadata_account,
                "master_edition_account" => master_edition_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
