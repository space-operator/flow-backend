use crate::prelude::*;
use anchor_lang_26::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, system_program};
use solana_sdk::pubkey::Pubkey;

use super::MetadataBubblegum;

// Command Name
const MINT_COMPRESSED_NFT: &str = "mint_compressed_NFT";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/compression/mint_compressed_NFT.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(MINT_COMPRESSED_NFT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(MINT_COMPRESSED_NFT, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub tree_delegate: Keypair,
    #[serde(with = "value::pubkey")]
    pub tree_authority: Pubkey,
    #[serde(with = "value::pubkey")]
    pub merkle_tree: Pubkey,
    #[serde(with = "value::pubkey")]
    pub leaf_owner: Pubkey,
    #[serde(with = "value::pubkey")]
    pub leaf_delegate: Pubkey,
    pub metadata: MetadataBubblegum,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let accounts = mpl_bubblegum::accounts::MintV1 {
        payer: input.payer.pubkey(),
        tree_authority: input.tree_authority,
        merkle_tree: input.merkle_tree,
        leaf_owner: input.leaf_owner,
        leaf_delegate: input.leaf_delegate,
        tree_delegate: input.tree_delegate.pubkey(),
        log_wrapper: spl_noop::id(),
        system_program: system_program::id(),
        compression_program: spl_account_compression::id(),
    }
    .to_account_metas(None);

    let metadata = input.metadata.into();

    let data = mpl_bubblegum::instruction::MintV1 { message: metadata }.data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(
            std::mem::size_of::<mpl_bubblegum::accounts::MintV1>(),
        )
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.tree_delegate.clone_keypair(),
        ]
        .into(),
        instructions: [Instruction {
            program_id: mpl_bubblegum::id(),
            accounts,
            data,
        }]
        .into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
