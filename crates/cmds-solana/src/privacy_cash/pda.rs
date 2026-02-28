use solana_program::pubkey::Pubkey;

/// Privacy Cash program ID (mainnet).
pub fn program_id() -> Pubkey {
    "9fhQBbumKEFuXtMBDw8AaQyAjCorLGJQiS3skWZdQyQD"
        .parse()
        .unwrap()
}

/// Privacy Cash program ID (devnet).
pub fn devnet_program_id() -> Pubkey {
    "ATZj4jZ4FFzkvAcvk27DW9GRkgSbFnHo49fKKPQXU7VS"
        .parse()
        .unwrap()
}

/// SOL merkle tree PDA: seeds = ["merkle_tree"]
pub fn find_merkle_tree() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"merkle_tree"], &program_id())
}

/// SPL token merkle tree PDA: seeds = ["merkle_tree", mint]
pub fn find_merkle_tree_spl(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"merkle_tree", mint.as_ref()], &program_id())
}

/// Tree token account PDA: seeds = ["tree_token"]
pub fn find_tree_token() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"tree_token"], &program_id())
}

/// Global config PDA: seeds = ["global_config"]
pub fn find_global_config() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"global_config"], &program_id())
}

/// Nullifier 0 PDA: seeds = ["nullifier0", nullifier_hash]
pub fn find_nullifier0(nullifier_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"nullifier0", nullifier_hash.as_ref()], &program_id())
}

/// Nullifier 1 PDA: seeds = ["nullifier1", nullifier_hash]
pub fn find_nullifier1(nullifier_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"nullifier1", nullifier_hash.as_ref()], &program_id())
}

// ── Devnet PDA helpers (for tests) ──

/// SOL merkle tree PDA on devnet.
pub fn find_merkle_tree_devnet() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"merkle_tree"], &devnet_program_id())
}

/// Global config PDA on devnet.
pub fn find_global_config_devnet() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"global_config"], &devnet_program_id())
}

/// Tree token account PDA on devnet.
pub fn find_tree_token_devnet() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"tree_token"], &devnet_program_id())
}

/// SPL token merkle tree PDA on devnet.
pub fn find_merkle_tree_spl_devnet(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"merkle_tree", mint.as_ref()], &devnet_program_id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_ids_parse() {
        let mainnet = program_id();
        let devnet = devnet_program_id();
        assert_ne!(mainnet, devnet);
        assert_ne!(mainnet, Pubkey::default());
        assert_ne!(devnet, Pubkey::default());
    }

    #[test]
    fn test_pda_derivation_deterministic() {
        let (tree1, bump1) = find_merkle_tree();
        let (tree2, bump2) = find_merkle_tree();
        assert_eq!(tree1, tree2);
        assert_eq!(bump1, bump2);

        let (config1, _) = find_global_config();
        let (config2, _) = find_global_config();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_pda_derivation_unique_per_seed() {
        let (tree, _) = find_merkle_tree();
        let (token, _) = find_tree_token();
        let (config, _) = find_global_config();
        assert_ne!(tree, token);
        assert_ne!(tree, config);
        assert_ne!(token, config);
    }

    #[test]
    fn test_spl_tree_different_per_mint() {
        let mint_a: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        let mint_b: Pubkey = "So11111111111111111111111111111111111111112"
            .parse()
            .unwrap();
        let (tree_a, _) = find_merkle_tree_spl(&mint_a);
        let (tree_b, _) = find_merkle_tree_spl(&mint_b);
        assert_ne!(tree_a, tree_b);
    }

    #[test]
    fn test_nullifier_pdas_different_per_hash() {
        let hash_a = [1u8; 32];
        let hash_b = [2u8; 32];
        let (null0_a, _) = find_nullifier0(&hash_a);
        let (null0_b, _) = find_nullifier0(&hash_b);
        assert_ne!(null0_a, null0_b);

        // nullifier0 and nullifier1 with same hash differ (different seed prefix)
        let (null0, _) = find_nullifier0(&hash_a);
        let (null1, _) = find_nullifier1(&hash_a);
        assert_ne!(null0, null1);
    }

    #[test]
    fn test_devnet_pdas_differ_from_mainnet() {
        let (mainnet_tree, _) = find_merkle_tree();
        let (devnet_tree, _) = find_merkle_tree_devnet();
        assert_ne!(mainnet_tree, devnet_tree);

        let (mainnet_config, _) = find_global_config();
        let (devnet_config, _) = find_global_config_devnet();
        assert_ne!(mainnet_config, devnet_config);
    }

    #[tokio::test]
    #[ignore = "requires devnet RPC access"]
    async fn test_devnet_program_exists() {
        let client = solana_rpc_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
        let program = devnet_program_id();
        let account = client.get_account(&program).unwrap();
        assert!(account.executable, "devnet program should be executable");
    }

    #[tokio::test]
    #[ignore = "requires devnet RPC access"]
    async fn test_devnet_pdas_exist() {
        let client = solana_rpc_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );

        let (tree, _) = find_merkle_tree_devnet();
        let result = client.get_account(&tree);
        tracing::info!("devnet SOL tree PDA: {tree}, exists: {}", result.is_ok());

        let (config, _) = find_global_config_devnet();
        let result = client.get_account(&config);
        tracing::info!(
            "devnet global_config PDA: {config}, exists: {}",
            result.is_ok()
        );
    }
}
