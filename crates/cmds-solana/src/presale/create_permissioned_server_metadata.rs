use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, derive_server_metadata, discriminators};

const NAME: &str = "create_permissioned_server_metadata";
const DEFINITION: &str = flow_lib::node_definition!("presale/create_permissioned_server_metadata.jsonc");

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
    pub owner: Wallet,
    pub server_url: String,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    /// Derived permissioned server metadata PDA
    #[serde_as(as = "AsPubkey")]
    pub permissioned_server_metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = derive_event_authority();
    let permissioned_server_metadata = derive_server_metadata(&input.presale);

    let accounts = vec![
        AccountMeta::new_readonly(input.presale, false),
        AccountMeta::new(permissioned_server_metadata, false),
        AccountMeta::new(input.owner.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),
    ];

    let mut data = discriminators::CREATE_PERMISSIONED_SERVER_METADATA.to_vec();
    borsh::to_writer(&mut data, &input.server_url)?;

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.owner.clone()].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        permissioned_server_metadata,
    })
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
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "server_url" => "https://example.com/server",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
