use super::{DISC_AUTHORIZE, build_auctioneer_instruction, pda};
use crate::auction_house::pda as ah_pda;
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auctioneer_authorize";
const DEFINITION: &str = flow_lib::node_definition!("auctioneer/authorize.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    /// Must be the Auction House authority (signs, pays rent for the auctioneer_authority PDA).
    pub wallet: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
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
    pub auctioneer_authority: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wallet_pk = input.wallet.pubkey();
    let (auction_house, _) = ah_pda::find_auction_house(&wallet_pk, &input.treasury_mint);
    let (auctioneer_authority, _) = pda::find_auctioneer_authority(&auction_house);

    // Idempotent fast-path: if the auctioneer_authority PDA is already
    // initialized on-chain, authorize has already run. Return existing PDAs
    // so this flow can re-run as verification against an existing setup.
    if ctx
        .solana_client()
        .get_account(&auctioneer_authority)
        .await
        .is_ok()
    {
        return Ok(Output {
            signature: None,
            auction_house,
            auctioneer_authority,
        });
    }

    let accounts = vec![
        AccountMeta::new(wallet_pk, true),
        AccountMeta::new_readonly(auction_house, false),
        AccountMeta::new(auctioneer_authority, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    let ix = build_auctioneer_instruction(DISC_AUTHORIZE, accounts, vec![]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.wallet.clone()]
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
        auctioneer_authority,
    })
}

#[cfg(test)]
mod tests {
    use super::super::AUCTIONEER_PROGRAM_ID;
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        let pk = "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9";
        let kp = "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ";
        let input = value::map! {
            "fee_payer" => kp, "wallet" => kp,
            "treasury_mint" => pk, "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auctioneer_instruction(DISC_AUTHORIZE, vec![], vec![]);
        assert_eq!(ix.program_id, AUCTIONEER_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_AUTHORIZE);
        assert_eq!(ix.data.len(), 8);
    }
}
