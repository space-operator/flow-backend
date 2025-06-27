use super::{
    GetAssetResponse, MetadataBubblegum,
    types::asset::{Asset, AssetProof},
};
use crate::prelude::*;
use mpl_bubblegum::instructions::UpdateMetadataBuilder;
use solana_program::instruction::AccountMeta;
use std::str::FromStr;

// Command Name
const NAME: &str = "update_cNFT";

const DEFINITION: &str = flow_lib::node_definition!("compression/update_cNFT.json");

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
    pub authority: Wallet,
    #[serde(default)]
    pub leaf_owner: Pubkey,
    //
    pub das_get_asset_proof: Option<GetAssetResponse<AssetProof>>,
    pub das_get_asset: Option<GetAssetResponse<Asset>>,
    //
    #[serde(with = "value::pubkey")]
    pub leaf_delegate: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection_mint: Option<Pubkey>,
    //
    #[serde(default, with = "value::pubkey::opt")]
    pub merkle_tree: Option<Pubkey>,
    pub root: Option<String>,
    pub leaf_id: Option<u64>,
    pub index: Option<u32>,
    pub proof: Option<Vec<String>>,
    //
    pub current_metadata: MetadataBubblegum,
    pub updated_metadata: MetadataBubblegum,
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
    // get from asset proof: merkle tree, root, index, proof
    // get from asset: data hash, creator hash, leaf id or nonce, metadata

    // Get proof
    let proof = match input.proof {
        Some(proof) => proof.to_owned(),
        None => match &input.das_get_asset_proof {
            Some(proof) => proof.result.proof.to_owned(),
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

    let _proof = proof?;

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

    let ix = match input.collection_mint {
        Some(collection_mint) => {
            let collection_authority_record_pda =
                mpl_token_metadata::accounts::CollectionAuthorityRecord::find_pda(
                    &collection_mint,
                    &input.authority.pubkey(),
                )
                .0;

            let collection_metadata =
                mpl_token_metadata::accounts::Metadata::find_pda(&collection_mint).0;

            UpdateMetadataBuilder::new()
                .payer(input.payer.pubkey())
                .tree_config(tree_config)
                .authority(input.authority.pubkey())
                .collection_mint(Some(collection_mint))
                .collection_metadata(Some(collection_metadata))
                .collection_authority_record_pda(Some(collection_authority_record_pda))
                .leaf_owner(input.leaf_owner)
                .leaf_delegate(input.leaf_delegate)
                .merkle_tree(input.merkle_tree.unwrap())
                .root(root)
                .nonce(nonce)
                .index(index)
                .update_args(input.updated_metadata.into())
                .instruction()
        }
        None => UpdateMetadataBuilder::new()
            .payer(input.payer.pubkey())
            .tree_config(tree_config)
            .authority(input.authority.pubkey())
            .leaf_owner(input.leaf_owner)
            .leaf_delegate(input.leaf_delegate)
            .merkle_tree(input.merkle_tree.unwrap())
            .root(root)
            .nonce(nonce)
            .index(index)
            .update_args(input.updated_metadata.into())
            .instruction(),
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.authority].into(),
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
