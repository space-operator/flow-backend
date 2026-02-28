use super::{JUP_LOCK_PROGRAM_ID, borsh_string, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "create_vesting_escrow_metadata";
const DEFINITION: &str = flow_lib::node_definition!("jup_lock/create_vesting_escrow_metadata.jsonc");

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
    pub creator: Wallet,
    pub payer: Wallet,
    // CreateVestingEscrowMetadataParameters
    pub name: String,
    pub description: String,
    pub creator_email: String,
    pub recipient_email: String,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub escrow_metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive escrow_metadata PDA from escrow
    let (escrow_metadata, _) = pda::find_escrow_metadata(&input.escrow);

    let accounts = vec![
        AccountMeta::new(input.escrow, false),
        AccountMeta::new_readonly(input.creator.pubkey(), true),
        AccountMeta::new(escrow_metadata, false),
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    // Borsh-serialize CreateVestingEscrowMetadataParameters
    let mut args_data = Vec::new();
    args_data.extend(borsh_string(&input.name));
    args_data.extend(borsh_string(&input.description));
    args_data.extend(borsh_string(&input.creator_email));
    args_data.extend(borsh_string(&input.recipient_email));

    let instruction = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"create_vesting_escrow_metadata", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.creator.clone(),
            input.payer.clone(),
        ]
        .into_iter()
        .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        escrow_metadata,
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
            "creator" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "name" => "Test Escrow",
            "description" => "A test vesting escrow",
            "creator_email" => "creator@test.com",
            "recipient_email" => "recipient@test.com",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_construction() {
        let escrow = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let (escrow_metadata, _) = pda::find_escrow_metadata(&escrow);

        let accounts = vec![
            AccountMeta::new(escrow, false),
            AccountMeta::new_readonly(creator, true),
            AccountMeta::new(escrow_metadata, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ];

        let mut args_data = Vec::new();
        args_data.extend(borsh_string("Test Escrow"));
        args_data.extend(borsh_string("A test vesting escrow"));
        args_data.extend(borsh_string("creator@test.com"));
        args_data.extend(borsh_string("recipient@test.com"));

        let ix = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"create_vesting_escrow_metadata", accounts, args_data);

        assert_eq!(ix.program_id, JUP_LOCK_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 5);
        // Discriminator (8) + 4 borsh strings with known lengths
        let expected_args_len = (4 + 11) + (4 + 21) + (4 + 16) + (4 + 18); // 82 bytes
        assert_eq!(ix.data.len(), 8 + expected_args_len);
    }

    #[test]
    fn test_metadata_args_serialization() {
        let mut args = Vec::new();
        args.extend(borsh_string("name"));
        args.extend(borsh_string("desc"));
        args.extend(borsh_string("a@b.com"));
        args.extend(borsh_string("c@d.com"));
        // 4 strings: (4+4) + (4+4) + (4+7) + (4+7) = 38 bytes
        assert_eq!(args.len(), 38);
    }
}
