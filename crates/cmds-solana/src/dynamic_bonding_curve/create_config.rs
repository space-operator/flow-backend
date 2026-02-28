use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "create_config";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/create_config.jsonc");

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
    pub config: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub fee_claimer: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub leftover_receiver: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    /// Config parameters as JSON
    pub params: JsonValue,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();

    let accounts = vec![
        // 0: config (writable, signer - new keypair)
        AccountMeta::new(input.config.pubkey(), true),
        // 1: fee_claimer (readonly)
        AccountMeta::new_readonly(input.fee_claimer, false),
        // 2: leftover_receiver (readonly)
        AccountMeta::new_readonly(input.leftover_receiver, false),
        // 3: quote_mint (readonly)
        AccountMeta::new_readonly(input.quote_mint, false),
        // 4: payer (writable, signer)
        AccountMeta::new(input.payer.pubkey(), true),
        // 5: system_program (readonly)
        AccountMeta::new_readonly(input.system_program, false),
        // 6: event_authority (readonly, PDA)
        AccountMeta::new_readonly(event_authority, false),
        // 7: program (readonly)
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    let mut data = discriminators::CREATE_CONFIG.to_vec();
    let params_bytes: Vec<u8> = serde_json::from_value(input.params)?;
    data.extend(params_bytes);

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.config, input.payer].into(),
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
}
