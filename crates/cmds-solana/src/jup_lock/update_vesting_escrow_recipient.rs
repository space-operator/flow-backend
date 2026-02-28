use crate::{
    jup_lock::{JUP_LOCK_PROGRAM_ID, borsh_option_string, pda},
    prelude::*,
};
use solana_program::instruction::AccountMeta;

const NAME: &str = "update_vesting_escrow_recipient";
const DEFINITION: &str =
    flow_lib::node_definition!("jup_lock/update_vesting_escrow_recipient.jsonc");

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
    pub signer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub new_recipient: Pubkey,
    #[serde(default)]
    pub new_recipient_email: Option<String>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Auto-derive escrow_metadata PDA. When new_recipient_email is provided,
    // pass the real PDA so the program updates the email on-chain.
    // When absent, use program ID sentinel (Anchor optional-account convention).
    let escrow_metadata_key = if input.new_recipient_email.is_some() {
        let (pda, _) = pda::find_escrow_metadata(&input.escrow);
        pda
    } else {
        JUP_LOCK_PROGRAM_ID
    };

    let accounts = vec![
        AccountMeta::new(input.escrow, false),
        AccountMeta::new(escrow_metadata_key, false),
        AccountMeta::new(input.signer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    // Borsh-serialize args: new_recipient (Pubkey) + new_recipient_email (Option<String>)
    let mut args_data = Vec::with_capacity(32 + 1 + 64);
    args_data.extend_from_slice(input.new_recipient.as_ref());
    args_data.extend(borsh_option_string(&input.new_recipient_email));

    let instruction = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"update_vesting_escrow_recipient", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.signer.clone()]
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
    fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "signer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "new_recipient" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_input_parsing_with_email() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "signer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "new_recipient" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "new_recipient_email" => "new@test.com",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_without_email() {
        let escrow = Pubkey::new_unique();
        let signer = Pubkey::new_unique();
        let new_recipient = Pubkey::new_unique();

        // Without email: escrow_metadata_key = JUP_LOCK_PROGRAM_ID (sentinel)
        let accounts = vec![
            AccountMeta::new(escrow, false),
            AccountMeta::new(JUP_LOCK_PROGRAM_ID, false),
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ];

        let mut args_data = Vec::with_capacity(33);
        args_data.extend_from_slice(new_recipient.as_ref());
        args_data.extend(borsh_option_string(&None));

        let ix = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"update_vesting_escrow_recipient", accounts, args_data);

        assert_eq!(ix.program_id, JUP_LOCK_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 4);
        // 8 discriminator + 32 pubkey + 1 None tag
        assert_eq!(ix.data.len(), 41);
    }

    #[test]
    fn test_instruction_with_email() {
        let escrow = Pubkey::new_unique();
        let signer = Pubkey::new_unique();
        let new_recipient = Pubkey::new_unique();
        let (escrow_metadata, _) = pda::find_escrow_metadata(&escrow);

        // With email: escrow_metadata_key = real PDA
        let accounts = vec![
            AccountMeta::new(escrow, false),
            AccountMeta::new(escrow_metadata, false),
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ];

        let email = "new@test.com".to_string();
        let mut args_data = Vec::new();
        args_data.extend_from_slice(new_recipient.as_ref());
        args_data.extend(borsh_option_string(&Some(email.clone())));

        let ix = crate::utils::build_anchor_instruction(JUP_LOCK_PROGRAM_ID,"update_vesting_escrow_recipient", accounts, args_data);

        assert_eq!(ix.program_id, JUP_LOCK_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 4);
        // 8 discriminator + 32 pubkey + 1 Some tag + 4 length + 12 email bytes
        assert_eq!(ix.data.len(), 8 + 32 + 1 + 4 + email.len());
    }

    #[test]
    fn test_sentinel_vs_pda_metadata_key() {
        let escrow = Pubkey::new_unique();
        let (real_metadata, _) = pda::find_escrow_metadata(&escrow);
        // Without email, the sentinel (program ID) should be used, not the PDA
        assert_ne!(
            JUP_LOCK_PROGRAM_ID, real_metadata,
            "Sentinel and real PDA must differ"
        );
    }
}
