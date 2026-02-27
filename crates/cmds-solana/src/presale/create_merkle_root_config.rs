use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, discriminators, CreateMerkleRootConfigParams};

const NAME: &str = "create_merkle_root_config";
const DEFINITION: &str = flow_lib::node_definition!("presale/create_merkle_root_config.jsonc");

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
    pub merkle_root_config: Pubkey,
    pub creator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    pub params: CreateMerkleRootConfigParams,
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
        AccountMeta::new_readonly(input.presale, false),           // presale (readonly)
        AccountMeta::new(input.merkle_root_config, false),         // merkle_root_config (writable, PDA)
        AccountMeta::new(input.creator.pubkey(), true),            // creator (writable, signer)
        AccountMeta::new_readonly(input.system_program, false),    // system_program (readonly)
        AccountMeta::new_readonly(event_authority, false),         // event_authority (PDA)
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),      // program
    ];

    let mut data = discriminators::CREATE_MERKLE_ROOT_CONFIG.to_vec();
    data.extend(borsh::to_vec(&input.params)?);

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator].into(),
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
            "merkle_root_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "creator" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "system_program" => "11111111111111111111111111111111",
            "params" => value::map! {
                "root" => vec![0u8; 32],
                "version" => 1u64,
            },
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
