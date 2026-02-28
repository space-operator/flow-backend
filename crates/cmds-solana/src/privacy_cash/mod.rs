//! Privacy Cash ZK mixer nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Privacy Cash program.
//! Program ID (mainnet): 9fhQBbumKEFuXtMBDw8AaQyAjCorLGJQiS3skWZdQyQD

pub mod helper;
pub mod pda;

// Admin instructions
pub mod initialize;
pub mod update_deposit_limit;
pub mod update_global_config;
pub mod init_spl_tree;
pub mod update_spl_deposit_limit;

// User instructions (ZK proof transactions)
pub mod transact;
pub mod transact_spl;
