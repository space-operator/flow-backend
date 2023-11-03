use crate::prelude::*;
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    instructions::CreateMasterEditionV3InstructionArgs,
};
use solana_program::system_program;

// Command Name
const NAME: &str = "create_master_edition";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/NFT/create_master_edition.json");

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
    update_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint_authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
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

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::accounts::MasterEdition,
        >())
        .await?;

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
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.update_authority.clone_keypair(),
        ]
        .into(),
        instructions: [create_ix].into(),
        minimum_balance_for_rent_exemption,
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
