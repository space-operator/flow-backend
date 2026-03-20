use super::{ESCROW_PROGRAM_ID, EscrowDiscriminator, build_escrow_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "add_timelock";
const DEFINITION: &str = flow_lib::node_definition!("escrow/add_timelock.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    pub lock_duration: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub extensions: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (extensions, bump) = pda::find_extensions(&input.escrow);
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(input.admin.pubkey(), true),
        AccountMeta::new_readonly(input.escrow, false),
        AccountMeta::new(extensions, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
    ];

    // Data: lock_duration (u64) + bump (u8)
    let mut args_data = Vec::with_capacity(9);
    args_data.extend_from_slice(&input.lock_duration.to_le_bytes());
    args_data.push(bump);

    let instruction =
        build_escrow_instruction(EscrowDiscriminator::AddTimelock, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.admin.clone()]
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
        extensions,
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
            "admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "lock_duration" => 86400u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_construction() {
        let escrow = Pubkey::new_unique();
        let (extensions, bump) = pda::find_extensions(&escrow);
        let (event_authority, _) = pda::find_event_authority();
        let fee_payer = Pubkey::new_unique();
        let admin = Pubkey::new_unique();

        let accounts = vec![
            AccountMeta::new(fee_payer, true),
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new_readonly(escrow, false),
            AccountMeta::new(extensions, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
        ];

        let mut args_data = Vec::with_capacity(9);
        args_data.extend_from_slice(&86400u64.to_le_bytes());
        args_data.push(bump);

        let ix = build_escrow_instruction(EscrowDiscriminator::AddTimelock, accounts, args_data);

        assert_eq!(ix.program_id, ESCROW_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 7);
        // 1 discriminator + 8 lock_duration + 1 bump = 10
        assert_eq!(ix.data.len(), 10);
        assert_eq!(ix.data[0], 1); // AddTimelock discriminator
    }
}
