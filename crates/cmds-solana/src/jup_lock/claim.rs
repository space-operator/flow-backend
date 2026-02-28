use super::{JUP_LOCK_PROGRAM_ID, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "claim";
const DEFINITION: &str = flow_lib::node_definition!("jup_lock/claim.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    pub recipient: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
    pub max_amount: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub escrow_token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub recipient_token: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive ATAs
    let (escrow_token, _) = pda::find_ata(&input.escrow, &input.mint, &input.token_program);
    let (recipient_token, _) = pda::find_ata(&input.recipient.pubkey(), &input.mint, &input.token_program);
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.escrow, false),
        AccountMeta::new(escrow_token, false),
        AccountMeta::new(input.recipient.pubkey(), true),
        AccountMeta::new(recipient_token, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(JUP_LOCK_PROGRAM_ID, false),
    ];

    let args_data = input.max_amount.to_le_bytes().to_vec();

    let instruction = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"claim", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.recipient.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        escrow_token,
        recipient_token,
    })
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
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "recipient" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "token_program" => "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "max_amount" => 1000u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_construction() {
        let escrow = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let token_program = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

        let (escrow_token, _) = pda::find_ata(&escrow, &mint, &token_program);
        let (recipient_token, _) = pda::find_ata(&recipient, &mint, &token_program);
        let (event_authority, _) = pda::find_event_authority();

        let accounts = vec![
            AccountMeta::new(escrow, false),
            AccountMeta::new(escrow_token, false),
            AccountMeta::new(recipient, true),
            AccountMeta::new(recipient_token, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(JUP_LOCK_PROGRAM_ID, false),
        ];

        let max_amount: u64 = 1_000_000;
        let args_data = max_amount.to_le_bytes().to_vec();
        let ix = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"claim", accounts, args_data);

        assert_eq!(ix.program_id, JUP_LOCK_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 7);
        // 8-byte discriminator + 8-byte u64 max_amount
        assert_eq!(ix.data.len(), 16);
    }

    #[test]
    fn test_derived_atas_differ() {
        let escrow = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

        let (escrow_token, _) = pda::find_ata(&escrow, &mint, &token_program);
        let (recipient_token, _) = pda::find_ata(&recipient, &mint, &token_program);
        assert_ne!(escrow_token, recipient_token, "Escrow and recipient ATAs must differ");
    }
}
