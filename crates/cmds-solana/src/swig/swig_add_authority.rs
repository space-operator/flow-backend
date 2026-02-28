use crate::prelude::*;
use super::{
    to_pubkey_v2, to_instruction_v3, build_client_action,
    AddAuthorityInstruction, AuthorityConfig, AuthorityType,
};

const NAME: &str = "swig_add_authority";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_add_authority.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub new_authority: Pubkey,
    #[serde(default = "default_permission")]
    pub permission_type: String,
    #[serde(default)]
    pub sol_limit_amount: Option<u64>,
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub token_mint: Option<Pubkey>,
    #[serde(default)]
    pub token_limit_amount: Option<u64>,
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub program_id_permission: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

fn default_permission() -> String { "all".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let actions = build_client_action(
        &input.permission_type,
        input.sol_limit_amount,
        input.token_mint.as_ref(),
        input.token_limit_amount,
        input.program_id_permission.as_ref(),
    );

    let new_authority_v2 = to_pubkey_v2(&input.new_authority);
    let ix_v2 = AddAuthorityInstruction::new_with_ed25519_authority(
        to_pubkey_v2(&input.swig_account),
        to_pubkey_v2(&input.fee_payer.pubkey()),
        to_pubkey_v2(&input.acting_authority.pubkey()),
        input.acting_role_id,
        AuthorityConfig {
            authority_type: AuthorityType::Ed25519,
            authority: new_authority_v2.as_ref(),
        },
        actions,
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
    fn test_instruction_builder_all() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let new_authority_v2 = to_pubkey_v2(&Keypair::new().pubkey());

        let ix = AddAuthorityInstruction::new_with_ed25519_authority(
            to_pubkey_v2(&swig_account),
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&kp.pubkey()),
            0,
            AuthorityConfig {
                authority_type: AuthorityType::Ed25519,
                authority: new_authority_v2.as_ref(),
            },
            build_client_action("all", None, None, None, None),
        ).unwrap();

        let instruction = to_instruction_v3(ix);
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert!(!instruction.data.is_empty());
    }

    #[test]
    fn test_instruction_builder_sol_limit() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let new_authority_v2 = to_pubkey_v2(&Keypair::new().pubkey());

        let ix = AddAuthorityInstruction::new_with_ed25519_authority(
            to_pubkey_v2(&swig_account),
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&kp.pubkey()),
            0,
            AuthorityConfig {
                authority_type: AuthorityType::Ed25519,
                authority: new_authority_v2.as_ref(),
            },
            build_client_action("sol_limit", Some(1_000_000_000), None, None, None),
        ).unwrap();

        let instruction = to_instruction_v3(ix);
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run_integration() {
        let wallet: Wallet = Keypair::new().into();
        let swig_account = Keypair::new().pubkey();
        let new_authority = Keypair::new().pubkey();

        let input = Input {
            fee_payer: wallet.clone(),
            swig_account,
            acting_authority: wallet,
            acting_role_id: 0,
            new_authority,
            permission_type: "all".to_string(),
            sol_limit_amount: None,
            token_mint: None,
            token_limit_amount: None,
            program_id_permission: None,
            submit: true,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
