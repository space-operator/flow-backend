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
use mpl_hybrid::instructions::{swap_nft_for_token_instruction};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

pub fn mpl_nft_to_token_swap(user_address: &str, nft_address: &str, token_amount: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Establish a connection to Solana blockchain (devnet or mainnet)
    let rpc_url = "https://api.devnet.solana.com"; // Update to mainnet if needed
    let connection = RpcClient::new(rpc_url);

    // Step 2: Convert the user address and NFT address to Solana Pubkeys
    let user_pubkey = Pubkey::from_str(user_address)?;
    let nft_pubkey = Pubkey::from_str(nft_address)?;

    // Step 3: Define the token swap logic (this could be based on your specific token or exchange mechanism)
    // Here we assume you have a function `swap_nft_for_token_instruction` that handles the swap logic
    let instruction = swap_nft_for_token_instruction(&connection, user_pubkey, nft_pubkey, token_amount)?;

    // Step 4: Send the transaction to the blockchain
    send_transaction(&connection, &instruction)?;

    Ok(())
}
