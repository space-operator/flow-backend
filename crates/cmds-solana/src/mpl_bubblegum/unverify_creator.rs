use super::{
    GetAssetResponse, MetadataBubblegum,
    types::asset::{Asset, AssetProof},
};
use crate::prelude::*;
use mpl_bubblegum::instructions::UnverifyCreatorBuilder;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

const NAME: &str = "unverify_creator_cNFT";

const DEFINITION: &str = flow_lib::node_definition!("mpl_bubblegum/unverify_creator.jsonc");

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
    pub creator: Wallet,
    #[serde(with = "value::pubkey")]
    pub leaf_owner: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub leaf_delegate: Option<Pubkey>,
    pub metadata: MetadataBubblegum,
    //
    pub das_get_asset_proof: Option<GetAssetResponse<AssetProof>>,
    pub das_get_asset: Option<GetAssetResponse<Asset>>,
    //
    #[serde(default, with = "value::pubkey::opt")]
    pub merkle_tree: Option<Pubkey>,
    pub root: Option<String>,
    pub data_hash: Option<String>,
    pub creator_hash: Option<String>,
    pub leaf_id: Option<u64>,
    pub index: Option<u32>,
    pub proof: Option<Vec<String>>,
    //
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Get proof
    let proof = match input.proof {
        Some(proof) => proof,
        None => match &input.das_get_asset_proof {
            Some(proof) => proof.result.proof.clone(),
            None => return Err(CommandError::msg("proof is required")),
        },
    };

    let proof: Result<Vec<AccountMeta>, CommandError> = proof
        .iter()
        .map(|node| {
            let pubkey =
                Pubkey::from_str(node).map_err(|_| CommandError::msg("Invalid pubkey string"))?;
            Ok(AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: false,
            })
        })
        .collect();

    let proof = proof?;

    // get root
    let root = match &input.root {
        Some(root) => root,
        None => match &input.das_get_asset_proof {
            Some(proof) => &proof.result.root,
            None => return Err(CommandError::msg("root is required")),
        },
    };

    let root = Pubkey::from_str(root)
        .map_err(|_| CommandError::msg("Invalid root string"))?
        .to_bytes();

    // get data hash
    let data_hash = match input.data_hash {
        Some(data_hash) => data_hash,
        None => match input.das_get_asset.clone() {
            Some(asset) => asset.result.compression.unwrap().data_hash,
            None => return Err(CommandError::msg("data_hash is required")),
        },
    };

    let data_hash = Pubkey::from_str(&data_hash)
        .map_err(|_| CommandError::msg("Invalid data_hash string"))?
        .to_bytes();

    // get creator hash
    let creator_hash = match input.creator_hash {
        Some(creator_hash) => creator_hash,
        None => match input.das_get_asset.clone() {
            Some(asset) => asset.result.compression.unwrap().creator_hash,
            None => return Err(CommandError::msg("creator_hash is required")),
        },
    };

    let creator_hash = Pubkey::from_str(&creator_hash)
        .map_err(|_| CommandError::msg("Invalid creator_hash string"))?
        .to_bytes();

    // leaf_id aka nonce
    let nonce = match input.leaf_id {
        Some(leaf_id) => leaf_id,
        None => match input.das_get_asset {
            Some(asset) => asset
                .result
                .compression
                .unwrap()
                .leaf_id
                .try_into()
                .unwrap(),
            None => return Err(CommandError::msg("leaf_id is required")),
        },
    };

    // get index
    let index = match input.index {
        Some(index) => index,
        None => match input.das_get_asset_proof.clone() {
            Some(asset) => (asset.result.node_index - 2 * asset.result.proof.len() as i64)
                .try_into()
                .unwrap(),
            None => return Err(CommandError::msg("index is required")),
        },
    };

    let merkle_tree = match input.merkle_tree {
        Some(merkle_tree) => merkle_tree,
        None => match input.das_get_asset_proof {
            Some(asset) => Pubkey::from_str(&asset.result.tree_id).unwrap(),
            None => return Err(CommandError::msg("merkle_tree is required")),
        },
    };

    let tree_config = mpl_bubblegum::accounts::TreeConfig::find_pda(&merkle_tree).0;
    let leaf_delegate = input.leaf_delegate.unwrap_or(input.leaf_owner);

    let ix = UnverifyCreatorBuilder::new()
        .tree_config(tree_config)
        .leaf_owner(input.leaf_owner)
        .leaf_delegate(leaf_delegate)
        .merkle_tree(merkle_tree)
        .payer(input.payer.pubkey())
        .creator(input.creator.pubkey())
        .root(root)
        .data_hash(data_hash)
        .creator_hash(creator_hash)
        .nonce(nonce)
        .index(index)
        .metadata(input.metadata.into())
        .add_remaining_accounts(&proof)
        .instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.creator].into(),
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
