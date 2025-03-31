use super::{
    types::asset::{Asset, AssetProof},
    GetAssetResponse, WalletOrPubkey,
};
use crate::prelude::*;
use mpl_bubblegum::instructions::TransferBuilder;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

// Command Name
const NAME: &str = "transfer_cNFT";

const DEFINITION: &str = flow_lib::node_definition!("compression/transfer.json");

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
    // #[serde(with = "value::pubkey")]
    // pub asset_id: Pubkey,
    #[serde(default)]
    pub leaf_owner: Option<WalletOrPubkey>,
    #[serde(with = "value::pubkey")]
    pub new_leaf_owner: Pubkey,
    //
    pub das_get_asset_proof: Option<GetAssetResponse<AssetProof>>,
    pub das_get_asset: Option<GetAssetResponse<Asset>>,
    //
    pub leaf_delegate: Option<Wallet>,
    #[serde(default, with = "value::pubkey::opt")]
    pub tree_config: Option<Pubkey>,
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

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
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

    // who is signing?
    let delegate_is_signing = input.leaf_delegate.is_some();

    let signer = match delegate_is_signing {
        true => input.leaf_delegate.as_ref().unwrap().clone(),
        false => match input.leaf_owner.as_ref().unwrap() {
            WalletOrPubkey::Wallet(k) => k.clone(),
            WalletOrPubkey::Pubkey(_) => {
                return Err(CommandError::msg("leaf delegate keypair required"));
            }
        },
    };

    let leaf_owner = match input.leaf_owner {
        Some(WalletOrPubkey::Wallet(k)) => k.pubkey(),
        Some(WalletOrPubkey::Pubkey(p)) => p,
        None => return Err(CommandError::msg("leaf_owner is required".to_string())),
    };

    // if delegate is signing, leaf delegate otherwise leaf owner
    let leaf_delegate = match delegate_is_signing {
        true => match input.leaf_delegate {
            Some(keypair) => keypair.pubkey(),
            None => return Err(CommandError::msg("leaf delegate keypair required")),
        },
        false => leaf_owner,
    };

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

    let tree_config = input
        .tree_config
        .unwrap_or(mpl_bubblegum::accounts::TreeConfig::find_pda(&merkle_tree).0);

    let mint_ix = TransferBuilder::new()
        .new_leaf_owner(input.new_leaf_owner)
        .tree_config(tree_config)
        .leaf_owner(leaf_owner, !delegate_is_signing)
        .leaf_delegate(leaf_delegate, delegate_is_signing)
        .merkle_tree(merkle_tree)
        //Optional with defaults
        // .log_wrapper(log_wrapper)
        // .compression_program(compression_program)
        // .system_program(system_program)
        .root(root)
        .data_hash(data_hash)
        .creator_hash(creator_hash)
        .nonce(nonce)
        .index(index)
        .add_remaining_accounts(&proof)
        .instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, signer].into(),
        instructions: [mint_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
