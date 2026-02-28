use crate::prelude::*;
use super::{to_pubkey_v2, to_instruction_v3, RemoveAuthorityInstruction};

const NAME: &str = "swig_remove_authority";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_remove_authority.jsonc");

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
    pub swig_account: Pubkey,
    pub acting_authority: Wallet,
    #[serde(default)]
    pub acting_role_id: u32,
    pub authority_to_remove_id: u32,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ix_v2 = RemoveAuthorityInstruction::new_with_ed25519_authority(
        to_pubkey_v2(&input.swig_account),
        to_pubkey_v2(&input.fee_payer.pubkey()),
        to_pubkey_v2(&input.acting_authority.pubkey()),
        input.acting_role_id,
        input.authority_to_remove_id,
    ).map_err(|e| CommandError::msg(e.to_string()))?;

    let instruction = to_instruction_v3(ix_v2);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.acting_authority].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swig::SWIG_PROGRAM_ID;
    use solana_keypair::{Keypair, Signer};

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_builder() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();

        let ix = RemoveAuthorityInstruction::new_with_ed25519_authority(
            to_pubkey_v2(&swig_account),
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&kp.pubkey()),
            0, 1,
        ).unwrap();

        let instruction = to_instruction_v3(ix);
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert!(!instruction.data.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run_integration() {
        let wallet: Wallet = Keypair::new().into();
        let swig_account = Keypair::new().pubkey();

        let input = Input {
            fee_payer: wallet.clone(),
            swig_account,
            acting_authority: wallet,
            acting_role_id: 0,
            authority_to_remove_id: 1,
            submit: true,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
