use super::{ETOKEN_PROGRAM_ID, MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "undelegate_ephemeral_ata";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/undelegate_ephemeral_ata.jsonc");

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
    pub user: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
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
    let ephemeral_ata = pda::ephemeral_ata(&input.user, &input.mint);
    let ata = spl_associated_token_account_interface::address::get_associated_token_address(
        &input.user,
        &input.mint,
    );

    let accounts = vec![
        AccountMeta::new_readonly(input.fee_payer.pubkey(), true), // fee_payer (signer, readonly)
        AccountMeta::new(ata, false),                              // ata (writable)
        AccountMeta::new_readonly(ephemeral_ata, false),           // ephemeral_ata PDA (readonly)
        AccountMeta::new(MAGIC_CONTEXT_ID, false),                 // magic_context (writable)
        AccountMeta::new_readonly(MAGIC_PROGRAM_ID, false),        // magic_program (readonly)
    ];

    let data = discriminators::UNDELEGATE_EPHEMERAL_ATA.to_vec();

    let instruction = Instruction {
        program_id: ETOKEN_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
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
}
