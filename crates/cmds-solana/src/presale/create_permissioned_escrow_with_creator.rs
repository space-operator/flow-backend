use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, discriminators, CreatePermissionedEscrowWithCreatorParams};

const NAME: &str = "create_permissioned_escrow_with_creator";
const DEFINITION: &str = flow_lib::node_definition!("presale/create_permissioned_escrow_with_creator.jsonc");

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
    pub presale: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub operator: Pubkey,
    pub operator_owner: Wallet,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    pub params: CreatePermissionedEscrowWithCreatorParams,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.presale, false),                        // presale (writable)
        AccountMeta::new(input.escrow, false),                         // escrow (writable, PDA)
        AccountMeta::new_readonly(input.owner, false),                 // owner (readonly)
        AccountMeta::new_readonly(input.operator, false),              // operator (readonly)
        AccountMeta::new_readonly(input.operator_owner.pubkey(), true), // operator_owner (signer)
        AccountMeta::new(input.payer.pubkey(), true),                  // payer (writable, signer)
        AccountMeta::new_readonly(input.system_program, false),        // system_program (readonly)
        AccountMeta::new_readonly(event_authority, false),             // event_authority (PDA)
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),          // program
    ];

    let mut data = discriminators::CREATE_PERMISSIONED_ESCROW_WITH_CREATOR.to_vec();
    data.extend(borsh::to_vec(&input.params)?);

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.operator_owner, input.payer].into(),
        instructions: [instruction].into(),
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

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "presale" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "owner" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "operator" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "operator_owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "system_program" => "11111111111111111111111111111111",
            "params" => value::map! {
                "registry_index" => 0u64,
                "deposit_cap" => 1000000u64,
                "padding" => vec![0u8; 32],
            },
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
