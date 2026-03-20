use super::{
    ESCROW_PROGRAM_ID, EscrowDiscriminator, build_escrow_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "escrow_deposit";
const DEFINITION: &str = flow_lib::node_definition!("escrow/deposit.jsonc");

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
    pub depositor: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    pub receipt_seed: Wallet,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
    pub amount: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub receipt: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (allowed_mint, _) = pda::find_allowed_mint(&input.escrow, &input.mint);
    let (receipt, receipt_bump) = pda::find_receipt(
        &input.escrow,
        &input.depositor.pubkey(),
        &input.mint,
        &input.receipt_seed.pubkey(),
    );
    let (vault, _) = pda::find_ata(&input.escrow, &input.mint, &input.token_program);
    let depositor_token_account =
        pda::find_ata(&input.depositor.pubkey(), &input.mint, &input.token_program).0;
    let (extensions, _) = pda::find_extensions(&input.escrow);
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(input.depositor.pubkey(), true),
        AccountMeta::new_readonly(input.escrow, false),
        AccountMeta::new_readonly(allowed_mint, false),
        AccountMeta::new_readonly(input.receipt_seed.pubkey(), true),
        AccountMeta::new(receipt, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(depositor_token_account, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
        AccountMeta::new_readonly(extensions, false),
    ];

    // Data: amount (u64) + bump (u8)
    let mut args_data = Vec::with_capacity(9);
    args_data.extend_from_slice(&input.amount.to_le_bytes());
    args_data.push(receipt_bump);

    let instruction = build_escrow_instruction(EscrowDiscriminator::Deposit, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.depositor.clone(),
            input.receipt_seed.clone(),
        ]
        .into_iter()
        .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature, receipt })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        // token_program omitted — defaults to SPL Token
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "depositor" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "receipt_seed" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "amount" => 1000u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
        let parsed = result.unwrap();
        assert_eq!(parsed.token_program, super::super::DEFAULT_TOKEN_PROGRAM);
    }

    #[test]
    fn test_instruction_construction() {
        let escrow = Pubkey::new_unique();
        let depositor = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let receipt_seed = Pubkey::new_unique();
        let token_program = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

        let (allowed_mint, _) = pda::find_allowed_mint(&escrow, &mint);
        let (receipt, receipt_bump) = pda::find_receipt(&escrow, &depositor, &mint, &receipt_seed);
        let (vault, _) = pda::find_ata(&escrow, &mint, &token_program);
        let depositor_token_account = pda::find_ata(&depositor, &mint, &token_program).0;
        let (extensions, _) = pda::find_extensions(&escrow);
        let (event_authority, _) = pda::find_event_authority();
        let fee_payer = Pubkey::new_unique();

        let accounts = vec![
            AccountMeta::new(fee_payer, true),
            AccountMeta::new_readonly(depositor, true),
            AccountMeta::new_readonly(escrow, false),
            AccountMeta::new_readonly(allowed_mint, false),
            AccountMeta::new_readonly(receipt_seed, true),
            AccountMeta::new(receipt, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(depositor_token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
            AccountMeta::new_readonly(extensions, false),
        ];

        let mut args_data = Vec::with_capacity(9);
        args_data.extend_from_slice(&1000u64.to_le_bytes());
        args_data.push(receipt_bump);

        let ix = build_escrow_instruction(EscrowDiscriminator::Deposit, accounts, args_data);

        assert_eq!(ix.program_id, ESCROW_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 14);
        // 1-byte discriminator + 8-byte amount + 1-byte bump = 10
        assert_eq!(ix.data.len(), 10);
        assert_eq!(ix.data[0], 3); // Deposit discriminator
    }
}
