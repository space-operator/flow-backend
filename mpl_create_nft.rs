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


use mpl_hybrid::instructions::create_nft_instruction;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

// Function that will call the `create_nft_instruction` to create an NFT on the blockchain.
pub fn mpl_create_nft(user_address: &str, nft_metadata: Metadata) -> Result<(), SomeErrorType> {
    // Step 1: Establish a connection to Solana's blockchain.
    let rpc_url = "https://api.devnet.solana.com";  // You can use mainnet or testnet here.
    let connection = RpcClient::new(rpc_url);

    // Step 2: Convert the user’s address into a Pubkey (Solana's key format)
    let user_pubkey = Pubkey::from_str(user_address)?;

    // Step 3: Call the `create_nft_instruction` and pass the necessary data.
    let instruction = create_nft_instruction(&connection, user_pubkey, nft_metadata)?;

    // Step 4: Send the transaction to the blockchain to actually create the NFT.
    send_transaction(&connection, &instruction)?;

    Ok(())
}
