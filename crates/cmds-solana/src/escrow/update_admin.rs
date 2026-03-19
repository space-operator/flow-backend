use super::{ESCROW_PROGRAM_ID, EscrowDiscriminator, build_escrow_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "update_admin";
const DEFINITION: &str = flow_lib::node_definition!("escrow/update_admin.jsonc");

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
    pub new_admin: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
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
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new_readonly(input.admin.pubkey(), true),
        AccountMeta::new_readonly(input.new_admin.pubkey(), true),
        AccountMeta::new(input.escrow, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
    ];

    let instruction =
        build_escrow_instruction(EscrowDiscriminator::UpdateAdmin, accounts, vec![]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.admin.clone(),
            input.new_admin.clone(),
        ]
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
    fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "new_admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_construction() {
        let admin = Pubkey::new_unique();
        let new_admin = Pubkey::new_unique();
        let escrow = Pubkey::new_unique();
        let (event_authority, _) = pda::find_event_authority();

        let accounts = vec![
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new_readonly(new_admin, true),
            AccountMeta::new(escrow, false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(ESCROW_PROGRAM_ID, false),
        ];

        let ix =
            build_escrow_instruction(EscrowDiscriminator::UpdateAdmin, accounts, vec![]);

        assert_eq!(ix.program_id, ESCROW_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 5);
        assert_eq!(ix.data.len(), 1); // discriminator only
        assert_eq!(ix.data[0], 4); // UpdateAdmin discriminator
    }
}
