use crate::prelude::*;
use solana_program::{pubkey::Pubkey, system_instruction::create_account};

use spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use std::mem::size_of;

const CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1: usize = 2 + 54;

// Command Name
const NAME: &str = "create_tree";

const DEFINITION: &str = flow_lib::node_definition!("compression/create_tree.json");

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
    pub merkle_tree: Wallet,
    pub max_depth: u32,
    pub max_buffer: u32,
    pub canopy_levels: Option<u32>,
    is_public: Option<bool>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let bubble_gum_program_id = mpl_bubblegum::ID;

    // Allocate tree's account

    // Only the following permutations are valid:
    let merkle_tree_account_size: usize = match input.max_depth {
        3 => match input.max_buffer {
            8 => {
                const MAX_DEPTH: usize = 3;
                const MAX_BUFFER_SIZE: usize = 8;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        5 => match input.max_buffer {
            8 => {
                const MAX_DEPTH: usize = 5;
                const MAX_BUFFER_SIZE: usize = 8;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        14 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 14;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            256 => {
                const MAX_DEPTH: usize = 14;
                const MAX_BUFFER_SIZE: usize = 256;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            1024 => {
                const MAX_DEPTH: usize = 14;
                const MAX_BUFFER_SIZE: usize = 1024;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            2048 => {
                const MAX_DEPTH: usize = 14;
                const MAX_BUFFER_SIZE: usize = 2048;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        15 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 15;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        16 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 16;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        17 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 17;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        18 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 18;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        19 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 19;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        20 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 20;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            256 => {
                const MAX_DEPTH: usize = 20;
                const MAX_BUFFER_SIZE: usize = 256;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            1024 => {
                const MAX_DEPTH: usize = 20;
                const MAX_BUFFER_SIZE: usize = 1024;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            2048 => {
                const MAX_DEPTH: usize = 20;
                const MAX_BUFFER_SIZE: usize = 2048;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        24 => match input.max_buffer {
            64 => {
                const MAX_DEPTH: usize = 24;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            256 => {
                const MAX_DEPTH: usize = 24;
                const MAX_BUFFER_SIZE: usize = 256;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            512 => {
                const MAX_DEPTH: usize = 24;
                const MAX_BUFFER_SIZE: usize = 512;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            1024 => {
                const MAX_DEPTH: usize = 24;
                const MAX_BUFFER_SIZE: usize = 1024;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            2048 => {
                const MAX_DEPTH: usize = 24;
                const MAX_BUFFER_SIZE: usize = 2048;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        26 => match input.max_buffer {
            512 => {
                const MAX_DEPTH: usize = 26;
                const MAX_BUFFER_SIZE: usize = 512;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            1024 => {
                const MAX_DEPTH: usize = 26;
                const MAX_BUFFER_SIZE: usize = 1024;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            2048 => {
                const MAX_DEPTH: usize = 26;
                const MAX_BUFFER_SIZE: usize = 2048;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },
        30 => match input.max_buffer {
            512 => {
                const MAX_DEPTH: usize = 30;
                const MAX_BUFFER_SIZE: usize = 512;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            1024 => {
                const MAX_DEPTH: usize = 30;
                const MAX_BUFFER_SIZE: usize = 1024;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            2048 => {
                const MAX_DEPTH: usize = 30;
                const MAX_BUFFER_SIZE: usize = 2048;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            _ => {
                return Err(anyhow::anyhow!("invalid max_buffer_size"));
            }
        },

        _ => {
            return Err(anyhow::anyhow!("invalid max_depth_size"));
        }
    };

    // To initialize a canopy on a ConcurrentMerkleTree account, you must initialize
    // the ConcurrentMerkleTree account with additional bytes. The number of additional bytes
    // needed is `(pow(2, N+1)-1) * 32`, where `N` is the number of levels of the merkle tree
    // you want the canopy to cache.
    //
    //https://github.com/solana-labs/solana-program-library/blob/9610bed5349f7a198903140cf2b74a727477b818/account-compression/programs/account-compression/src/canopy.rs
    //https://github.com/solana-labs/solana-program-library/blob/9610bed5349f7a198903140cf2b74a727477b818/account-compression/sdk/src/accounts/ConcurrentMerkleTreeAccount.ts#L209

    let canopy_size = match input.canopy_levels {
        Some(canopy_levels) => canopy_levels * 32,
        _ => 0,
    };

    let merkle_tree_account_size: usize =
        CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1 + merkle_tree_account_size + canopy_size as usize;

    let rent = ctx
        .solana_client()
        .get_minimum_balance_for_rent_exemption(merkle_tree_account_size)
        .await?;

    let create_merkle_account_ix = create_account(
        &input.payer.pubkey(),
        &input.merkle_tree.pubkey(),
        rent,
        u64::try_from(merkle_tree_account_size).unwrap(),
        &spl_account_compression::ID,
    );

    // Create Tree

    let pubkey = &input.merkle_tree.pubkey();
    let seeds = &[pubkey.as_ref()];
    let tree_config = Pubkey::find_program_address(seeds, &bubble_gum_program_id).0;

    let create_tree_config_ix = mpl_bubblegum::instructions::CreateTreeConfigBuilder::new()
        .tree_config(tree_config)
        .merkle_tree(input.merkle_tree.pubkey())
        .payer(input.payer.pubkey())
        .tree_creator(input.creator.pubkey())
        .max_depth(input.max_depth)
        .max_buffer_size(input.max_buffer)
        .public(input.is_public.is_some())
        .instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.creator, input.merkle_tree].into(),
        instructions: [create_merkle_account_ix, create_tree_config_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "tree_config" => tree_config,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
