use super::{
    ATA_PROGRAM_ID, DISC_UPDATE_AUCTION_HOUSE, TOKEN_PROGRAM_ID, build_auction_house_instruction,
    payment_account_for, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "auction_house_update";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/update_auction_house.jsonc");

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
    pub payer: Wallet,
    pub authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub new_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub fee_withdrawal_destination: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_withdrawal_destination_owner: Pubkey,
    #[serde(default)]
    pub seller_fee_basis_points: Option<u16>,
    #[serde(default)]
    pub requires_sign_off: Option<bool>,
    #[serde(default)]
    pub can_change_sale_price: Option<bool>,
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
    pub treasury_withdrawal_destination: Pubkey,
}

/// Borsh encoding for `Option<T>`: 1-byte tag (0 = None, 1 = Some) + payload.
fn encode_opt_u16(v: Option<u16>, out: &mut Vec<u8>) {
    match v {
        None => out.push(0),
        Some(x) => {
            out.push(1);
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
}

fn encode_opt_bool(v: Option<bool>, out: &mut Vec<u8>) {
    match v {
        None => out.push(0),
        Some(x) => {
            out.push(1);
            out.push(x as u8);
        }
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (auction_house, _) =
        pda::find_auction_house(&input.authority.pubkey(), &input.treasury_mint);
    let treasury_withdrawal_destination = payment_account_for(
        &input.treasury_withdrawal_destination_owner,
        &input.treasury_mint,
        &TOKEN_PROGRAM_ID,
    );

    let accounts = vec![
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new_readonly(input.payer.pubkey(), true),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new_readonly(input.new_authority, false),
        AccountMeta::new(input.fee_withdrawal_destination, false),
        AccountMeta::new(treasury_withdrawal_destination, false),
        AccountMeta::new_readonly(input.treasury_withdrawal_destination_owner, false),
        AccountMeta::new(auction_house, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
    ];

    let mut args_data = Vec::new();
    encode_opt_u16(input.seller_fee_basis_points, &mut args_data);
    encode_opt_bool(input.requires_sign_off, &mut args_data);
    encode_opt_bool(input.can_change_sale_price, &mut args_data);

    let ix = build_auction_house_instruction(DISC_UPDATE_AUCTION_HOUSE, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.payer.clone(),
            input.authority.clone(),
        ]
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
        treasury_withdrawal_destination,
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
        let input = value::map! {
            "fee_payer" => kp, "payer" => kp, "authority" => kp,
            "new_authority" => pk, "treasury_mint" => pk,
            "fee_withdrawal_destination" => pk,
            "treasury_withdrawal_destination_owner" => pk,
            "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_opt_encoding() {
        let mut v = Vec::new();
        encode_opt_u16(None, &mut v);
        encode_opt_u16(Some(500), &mut v);
        encode_opt_bool(None, &mut v);
        encode_opt_bool(Some(true), &mut v);
        // None(u16): [0]
        // Some(500): [1, 0xf4, 0x01]
        // None(bool): [0]
        // Some(true): [1, 1]
        assert_eq!(v, vec![0, 1, 0xf4, 0x01, 0, 1, 1]);
    }

    #[test]
    fn test_instruction_construction() {
        let ix = build_auction_house_instruction(DISC_UPDATE_AUCTION_HOUSE, vec![], vec![0, 0, 0]);
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_UPDATE_AUCTION_HOUSE);
        assert_eq!(ix.data.len(), 11);
    }
}
