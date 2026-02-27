use super::{JUP_LOCK_PROGRAM_ID, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "create_vesting_escrow";
const DEFINITION: &str = flow_lib::node_definition!("jup_lock/create_vesting_escrow.jsonc");

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
    pub base: Wallet,
    pub sender: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub recipient: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
    // CreateVestingEscrowParameters
    pub start_time: u64,
    pub frequency: u64,
    pub initial_unlock_amount: u64,
    pub amount_per_period: u64,
    pub number_of_period: u64,
    pub update_recipient_mode: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub escrow_token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub sender_token: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive PDAs
    let (escrow, _) = pda::find_escrow(&input.base.pubkey());
    let (escrow_token, _) = pda::find_ata(&escrow, &input.mint, &input.token_program);
    let (sender_token, _) =
        pda::find_ata(&input.sender.pubkey(), &input.mint, &input.token_program);
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.base.pubkey(), true),
        AccountMeta::new(escrow, false),
        AccountMeta::new(escrow_token, false),
        AccountMeta::new(input.sender.pubkey(), true),
        AccountMeta::new(sender_token, false),
        AccountMeta::new_readonly(input.recipient, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(JUP_LOCK_PROGRAM_ID, false),
    ];

    // Borsh-serialize CreateVestingEscrowParameters
    let mut args_data = Vec::with_capacity(41);
    args_data.extend_from_slice(&input.start_time.to_le_bytes());
    args_data.extend_from_slice(&input.frequency.to_le_bytes());
    args_data.extend_from_slice(&input.initial_unlock_amount.to_le_bytes());
    args_data.extend_from_slice(&input.amount_per_period.to_le_bytes());
    args_data.extend_from_slice(&input.number_of_period.to_le_bytes());
    args_data.push(input.update_recipient_mode);

    let instruction = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"create_vesting_escrow", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.base.clone(),
            input.sender.clone(),
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

    Ok(Output {
        signature,
        escrow,
        escrow_token,
        sender_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "base" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "sender" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "recipient" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_program" => "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "start_time" => 1000u64,
            "frequency" => 86400u64,
            "initial_unlock_amount" => 100u64,
            "amount_per_period" => 50u64,
            "number_of_period" => 12u64,
            "update_recipient_mode" => 0u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_construction() {
        let base = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let token_program = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

        let (escrow, _) = pda::find_escrow(&base);
        let (escrow_token, _) = pda::find_ata(&escrow, &mint, &token_program);
        let (sender_token, _) = pda::find_ata(&sender, &mint, &token_program);
        let (event_authority, _) = pda::find_event_authority();

        let accounts = vec![
            AccountMeta::new(base, true),
            AccountMeta::new(escrow, false),
            AccountMeta::new(escrow_token, false),
            AccountMeta::new(sender, true),
            AccountMeta::new(sender_token, false),
            AccountMeta::new_readonly(recipient, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(JUP_LOCK_PROGRAM_ID, false),
        ];

        // CreateVestingEscrowParameters: 5x u64 + 1x u8 = 41 bytes
        let mut args_data = Vec::with_capacity(41);
        args_data.extend_from_slice(&1000u64.to_le_bytes()); // start_time
        args_data.extend_from_slice(&86400u64.to_le_bytes()); // frequency
        args_data.extend_from_slice(&100u64.to_le_bytes()); // initial_unlock_amount
        args_data.extend_from_slice(&50u64.to_le_bytes()); // amount_per_period
        args_data.extend_from_slice(&12u64.to_le_bytes()); // number_of_period
        args_data.push(0u8); // update_recipient_mode

        let ix = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"create_vesting_escrow", accounts, args_data);

        assert_eq!(ix.program_id, JUP_LOCK_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 10);
        // 8-byte discriminator + 41-byte args
        assert_eq!(ix.data.len(), 49);
    }

    #[test]
    fn test_args_serialization_length() {
        // Vesting params: 5x u64 (40 bytes) + 1x u8 (1 byte) = 41 bytes
        let mut args = Vec::new();
        args.extend_from_slice(&0u64.to_le_bytes()); // start_time
        args.extend_from_slice(&0u64.to_le_bytes()); // frequency
        args.extend_from_slice(&0u64.to_le_bytes()); // initial_unlock_amount
        args.extend_from_slice(&0u64.to_le_bytes()); // amount_per_period
        args.extend_from_slice(&0u64.to_le_bytes()); // number_of_period
        args.push(0u8); // update_recipient_mode
        assert_eq!(args.len(), 41);
    }
}
