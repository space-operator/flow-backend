use super::{DISC_DELEGATE_AUCTIONEER, build_auction_house_instruction, pda};
use crate::auctioneer::pda as auctioneer_pda;
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auction_house_delegate_auctioneer";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/delegate_auctioneer.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Metaplex AuthorityScope enum — index into the AuctionHouse.scopes bool array.
/// Order is fixed by the on-chain enum definition.
#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum AuthorityScope {
    Deposit = 0,
    Buy = 1,
    PublicBuy = 2,
    ExecuteSale = 3,
    Sell = 4,
    Cancel = 5,
    Withdraw = 6,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    /// Optional override. Omit to delegate to the canonical `mpl-auctioneer` PDA for this AH.
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub auctioneer_authority: Option<Pubkey>,
    /// List of scope bytes. Omit to delegate all 7 scopes.
    #[serde(default)]
    pub scopes: Option<Vec<u8>>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub auction_house: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub ah_auctioneer_pda: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub auctioneer_authority: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let auctioneer_authority = input
        .auctioneer_authority
        .unwrap_or_else(|| auctioneer_pda::find_auctioneer_authority(&auction_house).0);
    let (ah_auctioneer_pda, _) = pda::find_ah_auctioneer_pda(&auction_house, &auctioneer_authority);

    // Idempotent fast-path: if the auctioneer has already been delegated to
    // this AH (ah_auctioneer_pda exists), skip the tx so this flow can re-run
    // as a verification step without re-submitting a duplicate delegation.
    if ctx
        .solana_client()
        .get_account(&ah_auctioneer_pda)
        .await
        .is_ok()
    {
        return Ok(Output {
            signature: None,
            auction_house,
            ah_auctioneer_pda,
            auctioneer_authority,
        });
    }

    let accounts = vec![
        AccountMeta::new(auction_house, false),
        AccountMeta::new(input.authority.pubkey(), true),
        AccountMeta::new_readonly(auctioneer_authority, false),
        AccountMeta::new(ah_auctioneer_pda, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    let scopes = input.scopes.unwrap_or_else(|| vec![0, 1, 2, 3, 4, 5, 6]);
    let mut args_data = Vec::with_capacity(4 + scopes.len());
    args_data.extend_from_slice(&(scopes.len() as u32).to_le_bytes());
    args_data.extend_from_slice(&scopes);

    let ix = build_auction_house_instruction(DISC_DELEGATE_AUCTIONEER, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.authority.clone()]
            .into_iter()
            .collect(),
        instructions: vec![ix],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        auction_house,
        ah_auctioneer_pda,
        auctioneer_authority,
    })
}

#[cfg(test)]
mod tests {
    use super::super::AUCTION_HOUSE_PROGRAM_ID;
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        let pk = "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9";
        let kp = "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ";
        // auctioneer_authority omitted → canonical mpl-auctioneer PDA
        let input = value::map! {
            "fee_payer" => kp, "authority" => kp,
            "treasury_mint" => pk,
            "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_scopes_encoding() {
        // Default scopes: all 7
        let scopes: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6];
        let mut data = Vec::new();
        data.extend_from_slice(&(scopes.len() as u32).to_le_bytes());
        data.extend_from_slice(&scopes);
        assert_eq!(&data[..4], &[7, 0, 0, 0]);
        assert_eq!(&data[4..], &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_instruction_construction() {
        let ix =
            build_auction_house_instruction(DISC_DELEGATE_AUCTIONEER, vec![], vec![0, 0, 0, 0]);
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_DELEGATE_AUCTIONEER);
        assert_eq!(ix.data.len(), 12);
    }
}
