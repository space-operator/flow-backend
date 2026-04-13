use super::{
    ATA_PROGRAM_ID, DISC_CREATE_AUCTION_HOUSE, TOKEN_PROGRAM_ID, build_auction_house_instruction,
    payment_account_for, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;

const NAME: &str = "auction_house_create";
const DEFINITION: &str = flow_lib::node_definition!("auction_house/create_auction_house.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub fee_withdrawal_destination: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_withdrawal_destination_owner: Pubkey,
    pub seller_fee_basis_points: u16,
    pub requires_sign_off: bool,
    pub can_change_sale_price: bool,
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
    pub auction_house_fee_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub auction_house_treasury: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_withdrawal_destination: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (auction_house, _) = pda::find_auction_house(&input.authority, &input.treasury_mint);
    let (auction_house_fee_account, _) = pda::find_auction_house_fee_account(&auction_house);
    let (auction_house_treasury, _) = pda::find_auction_house_treasury(&auction_house);
    let treasury_withdrawal_destination = payment_account_for(
        &input.treasury_withdrawal_destination_owner,
        &input.treasury_mint,
        &TOKEN_PROGRAM_ID,
    );

    let accounts = vec![
        AccountMeta::new_readonly(input.treasury_mint, false),
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new_readonly(input.authority, false),
        AccountMeta::new(input.fee_withdrawal_destination, false),
        AccountMeta::new_readonly(input.treasury_withdrawal_destination_owner, false),
        AccountMeta::new(treasury_withdrawal_destination, false),
        AccountMeta::new(auction_house, false),
        AccountMeta::new(auction_house_fee_account, false),
        AccountMeta::new(auction_house_treasury, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let mut args_data = Vec::with_capacity(4);
    args_data.extend_from_slice(&input.seller_fee_basis_points.to_le_bytes());
    args_data.push(input.requires_sign_off as u8);
    args_data.push(input.can_change_sale_price as u8);

    let ix = build_auction_house_instruction(DISC_CREATE_AUCTION_HOUSE, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.payer.clone()]
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
        auction_house_fee_account,
        auction_house_treasury,
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
            "fee_payer" => kp,
            "payer" => kp,
            "authority" => pk,
            "treasury_mint" => pk,
            "fee_withdrawal_destination" => pk,
            "treasury_withdrawal_destination_owner" => pk,
            "seller_fee_basis_points" => 500u64,
            "requires_sign_off" => false,
            "can_change_sale_price" => false,
            "submit" => false,
        };
        value::from_map::<Input>(input).unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let auth = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let (ah, _) = pda::find_auction_house(&auth, &mint);
        let (fee, _) = pda::find_auction_house_fee_account(&ah);
        let (tre, _) = pda::find_auction_house_treasury(&ah);
        let payer = Pubkey::new_unique();
        let dest = Pubkey::new_unique();

        let accounts = vec![
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(auth, false),
            AccountMeta::new(dest, false),
            AccountMeta::new_readonly(dest, false),
            AccountMeta::new(dest, false),
            AccountMeta::new(ah, false),
            AccountMeta::new(fee, false),
            AccountMeta::new(tre, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ];

        let mut args = Vec::new();
        args.extend_from_slice(&500u16.to_le_bytes());
        args.push(0);
        args.push(1);

        let ix = build_auction_house_instruction(DISC_CREATE_AUCTION_HOUSE, accounts, args);
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 13);
        // 8 disc + 2 u16 + 2 bools = 12
        assert_eq!(ix.data.len(), 12);
        assert_eq!(ix.data[..8], DISC_CREATE_AUCTION_HOUSE);
    }
}
