use crate::prelude::*;
use anchor_lang_26::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, system_instruction, system_program};
use solana_sdk::pubkey::Pubkey;
use spl_account_compression::{
    self, state::CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1, ConcurrentMerkleTree,
};
use std::mem::size_of;

// Command Name
const CREATE_TREE: &str = "create_tree";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/compression/create_tree.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(CREATE_TREE)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(CREATE_TREE, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub creator: Keypair,
    #[serde(with = "value::keypair")]
    pub merkle_tree: Keypair,
    pub max_depth: u32,
    pub max_buffer: u32,
    pub canopy_levels: Option<u32>,
    is_public: Option<bool>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let bubble_gum_program_id = mpl_bubblegum::id();

    // Allocate tree's account

    // Only the following pesrmutations are valid:
    //
    // | max_depth | max_buffer_size       |
    // | --------- | --------------------- |
    // | 14        | (64, 256, 1024, 2048) |
    // | 20        | (64, 256, 1024, 2048) |
    // | 24        | (64, 256, 512, 1024, 2048) |
    // | 26        | (64, 256, 512, 1024, 2048) |
    // | 30        | (512, 1024, 2048) |
    // const MAX_DEPTH: usize = 14;
    // const MAX_BUFFER_SIZE: usize = 64;
    let merkle_tree_account_size: usize = match input.max_depth {
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
            64 => {
                const MAX_DEPTH: usize = 26;
                const MAX_BUFFER_SIZE: usize = 64;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
            256 => {
                const MAX_DEPTH: usize = 26;
                const MAX_BUFFER_SIZE: usize = 256;
                size_of::<ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>>()
            }
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

    let canopy_size = if let Some(canopy_levels) = input.canopy_levels {
        canopy_levels * 32
    } else {
        0
    };

    let merkle_tree_account_size: usize =
        CONCURRENT_MERKLE_TREE_HEADER_SIZE_V1 + merkle_tree_account_size + canopy_size as usize;

    let lamports = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(merkle_tree_account_size)
        .await?;

    let create_account_tree_size = system_instruction::create_account(
        &input.payer.pubkey(),
        &input.merkle_tree.pubkey(),
        lamports,
        u64::try_from(merkle_tree_account_size).unwrap(),
        &spl_account_compression::id(),
    );

    // Create Tree

    let pubkey = &input.merkle_tree.pubkey();
    let seeds = &[pubkey.as_ref()];
    let tree_authority = Pubkey::find_program_address(seeds, &bubble_gum_program_id).0;

    let accounts = mpl_bubblegum::accounts::CreateTree {
        payer: input.payer.pubkey(),
        tree_authority,
        merkle_tree: input.merkle_tree.pubkey(),
        tree_creator: input.creator.pubkey(),
        log_wrapper: spl_noop::id(),
        system_program: system_program::id(),
        compression_program: spl_account_compression::id(),
    }
    .to_account_metas(None);

    let data = mpl_bubblegum::instruction::CreateTree {
        max_depth: input.max_depth,
        max_buffer_size: input.max_buffer,
        public: input.is_public,
    }
    .data();

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_bubblegum::accounts::CreateTree,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.creator.clone_keypair(),
            input.merkle_tree.clone_keypair(),
        ]
        .into(),
        instructions: [
            create_account_tree_size,
            Instruction {
                program_id: mpl_bubblegum::id(),
                accounts,
                data,
            },
        ]
        .into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "tree_authority" => tree_authority,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
