use super::{DBC_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "create_virtual_pool_metadata";
const DEFINITION: &str =
    flow_lib::node_definition!("dynamic_bonding_curve/create_virtual_pool_metadata.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// On-chain instruction argument: CreateVirtualPoolMetadataParameters
/// IDL fields: padding ([u8; 96]), name (String), website (String), logo (String)
#[derive(borsh::BorshSerialize, Debug)]
struct CreateVirtualPoolMetadataParameters {
    padding: [u8; 96],
    name: String,
    website: String,
    logo: String,
}

/// The `padding` port accepts a JSON object with { name, website, logo } fields.
/// These are encoded into the on-chain CreateVirtualPoolMetadataParameters struct.
#[derive(Serialize, Deserialize, Debug, Default)]
struct MetadataInput {
    #[serde(default)]
    name: String,
    #[serde(default)]
    website: String,
    #[serde(default)]
    logo: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    pub creator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    #[serde(default)]
    pub padding: MetadataInput,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub virtual_pool_metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let virtual_pool_metadata = pda::virtual_pool_metadata(&input.pool);

    // IDL account order:
    // 0: virtual_pool (writable)
    // 1: virtual_pool_metadata (writable, PDA)
    // 2: creator (signer, relation to virtual_pool)
    // 3: payer (writable, signer)
    // 4: system_program
    // 5: event_authority (PDA)
    // 6: program
    let accounts = vec![
        AccountMeta::new(input.pool, false),
        AccountMeta::new(virtual_pool_metadata, false),
        AccountMeta::new_readonly(input.creator.pubkey(), true),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(input.system_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    let metadata_params = CreateVirtualPoolMetadataParameters {
        padding: [0u8; 96],
        name: input.padding.name,
        website: input.padding.website,
        logo: input.padding.logo,
    };

    let mut data = discriminators::CREATE_VIRTUAL_POOL_METADATA.to_vec();
    data.extend(borsh::to_vec(&metadata_params)?);

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };
    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output {
        signature,
        virtual_pool_metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
}
