use solana_sdk::pubkey::Pubkey;

pub fn find_proxy_authority_address(authority: &Pubkey) -> Pubkey {
    let (expected_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"proxy", &authority.to_bytes()], &space_wrapper::ID);
    expected_pda
}
