use crate::prelude::*;
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata, TokenRecord},
    instructions::MintV1InstructionArgs,
};
use solana_program::sysvar;
use solana_sdk_ids::system_program;

use super::AuthorizationData;

// Command Name
const NAME: &str = "mint_v1";

const DEFINITION: &str = flow_lib::node_definition!("nft/v1/mint_v1.json");

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
    pub authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner: Pubkey,
    pub amount: u64,
    #[serde(default, with = "value::pubkey::opt")]
    pub delegate_record: Option<Pubkey>,
    #[serde(default, with = "value::pubkey::opt")]
    pub authorization_rules_program: Option<Pubkey>,
    #[serde(default, with = "value::pubkey::opt")]
    pub authorization_rules: Option<Pubkey>,
    pub authorization_data: Option<AuthorizationData>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let (master_edition_account, _) = MasterEdition::find_pda(&input.mint_account);

    // get associated token account pda
    let token_account = spl_associated_token_account::get_associated_token_address(
        &input.token_owner,
        &input.mint_account,
    );

    let token_record = TokenRecord::find_pda(&input.mint_account, &token_account).0;

    let create_ix = mpl_token_metadata::instructions::MintV1 {
        metadata: metadata_account,
        master_edition: Some(master_edition_account),
        mint: input.mint_account,
        authority: input.authority.pubkey(),
        payer: input.fee_payer.pubkey(),
        token: token_account,
        token_owner: Some(input.token_owner),
        token_record: Some(token_record),
        delegate_record: input.delegate_record,
        spl_ata_program: spl_associated_token_account::id(),
        authorization_rules_program: input.authorization_rules_program,
        authorization_rules: input.authorization_rules,
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
        spl_token_program: spl_token_interface::ID,
    };

    let args = MintV1InstructionArgs {
        amount: input.amount,
        authorization_data: input.authorization_data.map(Into::into),
    };

    let create_ix = create_ix.instruction(args);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [create_ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "token"=> token_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
