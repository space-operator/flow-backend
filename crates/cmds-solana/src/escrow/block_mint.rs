use super::{ESCROW_PROGRAM_ID, EscrowDiscriminator, build_escrow_instruction, default_token_program, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "block_mint";
const DEFINITION: &str = flow_lib::node_definition!("escrow/block_mint.jsonc");

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
    pub admin: Wallet,
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub rent_recipient: Option<Pubkey>,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
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
    let (allowed_mint, _) = pda::find_allowed_mint(&input.escrow, &input.mint);
    let (event_authority, _) = pda::find_event_authority();
    let rent_recipient = input.rent_recipient.unwrap_or_else(|| input.fee_payer.pubkey());

    let accounts = vec![
        AccountMeta::new_readonly(input.admin.pubkey(), true),
        AccountMeta::new_readonly(input.fee_payer.pubkey(), true),
        AccountMeta::new(rent_recipient, false),
        AccountMeta::new_readonly(input.escrow, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new(allowed_mint, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
    ];

    let instruction =
        build_escrow_instruction(EscrowDiscriminator::BlockMint, accounts, vec![]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.admin.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit { ins } else { Default::default() };
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
            "admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
        let parsed = result.unwrap();
        assert_eq!(parsed.token_program, super::super::DEFAULT_TOKEN_PROGRAM);
        assert!(parsed.rent_recipient.is_none()); // defaults to fee_payer at runtime
    }

    #[test]
    fn test_instruction_construction() {
        let escrow = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program =
            solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let (allowed_mint, _) = pda::find_allowed_mint(&escrow, &mint);
        let (event_authority, _) = pda::find_event_authority();
        let admin = Pubkey::new_unique();
        let fee_payer = Pubkey::new_unique();
        let rent_recipient = Pubkey::new_unique();

        let accounts = vec![
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new_readonly(fee_payer, true),
            AccountMeta::new(rent_recipient, false),
            AccountMeta::new_readonly(escrow, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(allowed_mint, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
        ];

        let ix = build_escrow_instruction(EscrowDiscriminator::BlockMint, accounts, vec![]);

        assert_eq!(ix.program_id, ESCROW_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 9);
        assert_eq!(ix.data.len(), 1); // discriminator only
        assert_eq!(ix.data[0], 7); // BlockMint discriminator
    }
}
