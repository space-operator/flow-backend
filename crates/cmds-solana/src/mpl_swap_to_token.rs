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

use mpl_hybrid::instructions::swap_to_token_instruction;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

// Function that calls the `swap_to_token_instruction` to swap an NFT for a token
pub fn mpl_swap_to_token(user_address: &str, token_data: TokenData) -> Result<(), SomeErrorType> {
    // Step 1: Establish a connection to the Solana blockchain.
    let rpc_url = "https://api.devnet.solana.com";  // You can use mainnet or testnet here.
    let connection = RpcClient::new(rpc_url);

    // Step 2: Convert user address into a Solana Pubkey.
    let user_pubkey = Pubkey::from_str(user_address)?;

    // Step 3: Call the `swap_to_token_instruction` and pass necessary data.
    let instruction = swap_to_token_instruction(&connection, user_pubkey, token_data)?;

    // Step 4: Send the transaction to the blockchain to complete the swap.
    send_transaction(&connection, &instruction)?;

    Ok(())
}
