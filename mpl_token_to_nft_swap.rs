host = "0.0.0.0"
port = 8080
local_storage = "_data/guest_local_storage"
cors_origins = ["*"]

[supabase]
anon_key = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Imh5amJvYmxramVldmt6YXFzeXhlIiwicm9sZSI6ImFub24iLCJpYXQiOjE3Mjc4ODc1MjgsImV4cCI6MjA0MzQ2MzUyOH0.J4tyAfa2_j1irMW3sZUi7ykTrubGdoqXWo9cPXqZ9iw"
endpoint = "https://hyjboblkjeevkzaqsyxe.supabase.co"

[db]
upstream_url = "https://dev-api.spaceoperator.com"
api_keys = ["b3-EGITlELGk29ZKg42LxFFIn80BpCWcFlM-C15kE0owOA"]

use mpl_hybrid::instructions::{swap_token_for_nft_instruction};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

pub fn mpl_token_to_nft_swap(user_address: &str, token_address: &str, nft_metadata: Metadata) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Establish a connection to Solana blockchain (devnet or mainnet)
    let rpc_url = "https://api.devnet.solana.com"; // Update to mainnet if needed
    let connection = RpcClient::new(rpc_url);

    // Step 2: Convert the user address and token address to Solana Pubkeys
    let user_pubkey = Pubkey::from_str(user_address)?;
    let token_pubkey = Pubkey::from_str(token_address)?;

    // Step 3: Define the logic for swapping token to NFT (specific token logic)
    let instruction = swap_token_for_nft_instruction(&connection, user_pubkey, token_pubkey, nft_metadata)?;

    // Step 4: Send the transaction to the blockchain
    send_transaction(&connection, &instruction)?;

    Ok(())
}
