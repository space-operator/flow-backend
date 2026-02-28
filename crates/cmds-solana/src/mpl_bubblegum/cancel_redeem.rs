use super::{
    GetAssetResponse,
    types::asset::{Asset, AssetProof},
};
use crate::prelude::*;
use mpl_bubblegum::instructions::CancelRedeemBuilder;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

const NAME: &str = "cancel_redeem_cNFT";

const DEFINITION: &str = flow_lib::node_definition!("mpl_bubblegum/cancel_redeem.jsonc");

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
    //
    pub das_get_asset_proof: Option<GetAssetResponse<AssetProof>>,
    pub das_get_asset: Option<GetAssetResponse<Asset>>,
    //
    #[serde(default, with = "value::pubkey::opt")]
    pub merkle_tree: Option<Pubkey>,
    pub root: Option<String>,
    pub leaf_id: Option<u64>,
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

    // leaf_id aka nonce - needed for voucher PDA derivation
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

    let merkle_tree = match input.merkle_tree {
        Some(merkle_tree) => merkle_tree,
        None => match input.das_get_asset_proof {
            Some(asset) => Pubkey::from_str(&asset.result.tree_id).unwrap(),
            None => return Err(CommandError::msg("merkle_tree is required")),
        },
    };

    let tree_config = mpl_bubblegum::accounts::TreeConfig::find_pda(&merkle_tree).0;
    let voucher = mpl_bubblegum::accounts::Voucher::find_pda(&merkle_tree, nonce).0;

    let ix = CancelRedeemBuilder::new()
        .tree_config(tree_config)
        .leaf_owner(input.leaf_owner.pubkey())
        .merkle_tree(merkle_tree)
        .voucher(voucher)
        .root(root)
        .add_remaining_accounts(&proof)
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
