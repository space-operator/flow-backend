use super::{
    ESCROW_PROGRAM_ID, EscrowDiscriminator, build_escrow_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "escrow_withdraw";
const DEFINITION: &str = flow_lib::node_definition!("escrow/withdraw.jsonc");

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
    pub withdrawer: Wallet,
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub rent_recipient: Option<Pubkey>,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub receipt: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
    /// Optional arbiter wallet. Required when the escrow has an arbiter extension
    /// set via `set_arbiter`. The arbiter must sign to authorize the withdrawal.
    #[serde(default)]
    pub arbiter: Option<Wallet>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (extensions, _) = pda::find_extensions(&input.escrow);
    let (vault, _) = pda::find_ata(&input.escrow, &input.mint, &input.token_program);
    let withdrawer_token_account = pda::find_ata(
        &input.withdrawer.pubkey(),
        &input.mint,
        &input.token_program,
    )
    .0;
    let (event_authority, _) = pda::find_event_authority();
    let rent_recipient = input
        .rent_recipient
        .unwrap_or_else(|| input.fee_payer.pubkey());

    let mut accounts = vec![
        AccountMeta::new(rent_recipient, false),
        AccountMeta::new_readonly(input.withdrawer.pubkey(), true),
        AccountMeta::new_readonly(input.escrow, false),
        AccountMeta::new_readonly(extensions, false),
        AccountMeta::new(input.receipt, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(withdrawer_token_account, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
    ];

    // When an arbiter extension is set on the escrow, the arbiter must be passed
    // as the first remaining account and must sign to authorize the withdrawal.
    if let Some(ref arbiter) = input.arbiter {
        accounts.push(AccountMeta::new_readonly(arbiter.pubkey(), true));
    }

    let instruction = build_escrow_instruction(EscrowDiscriminator::Withdraw, accounts, vec![]);

    let mut signers: std::collections::BTreeSet<Wallet> =
        [input.fee_payer.clone(), input.withdrawer.clone()]
            .into_iter()
            .collect();
    if let Some(ref arbiter) = input.arbiter {
        signers.insert(arbiter.clone());
    }

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers,
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing_minimal() {
        // Only required inputs — token_program and rent_recipient default
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "withdrawer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "receipt" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
        let parsed = result.unwrap();
        assert_eq!(parsed.token_program, super::super::DEFAULT_TOKEN_PROGRAM);
        assert!(parsed.rent_recipient.is_none());
    }

    #[test]
    fn test_instruction_construction() {
        let escrow = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let (extensions, _) = pda::find_extensions(&escrow);
        let (vault, _) = pda::find_ata(&escrow, &mint, &token_program);
        let withdrawer = Pubkey::new_unique();
        let withdrawer_token_account = pda::find_ata(&withdrawer, &mint, &token_program).0;
        let (event_authority, _) = pda::find_event_authority();
        let fee_payer = Pubkey::new_unique();
        let rent_recipient = Pubkey::new_unique();
        let receipt = Pubkey::new_unique();

        let accounts = vec![
            AccountMeta::new(rent_recipient, false),
            AccountMeta::new_readonly(withdrawer, true),
            AccountMeta::new_readonly(escrow, false),
            AccountMeta::new_readonly(extensions, false),
            AccountMeta::new(receipt, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(withdrawer_token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
        ];

        let ix = build_escrow_instruction(EscrowDiscriminator::Withdraw, accounts, vec![]);

        assert_eq!(ix.program_id, ESCROW_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 12);
        assert_eq!(ix.data.len(), 1); // discriminator only
        assert_eq!(ix.data[0], 5); // Withdraw discriminator
    }
}
