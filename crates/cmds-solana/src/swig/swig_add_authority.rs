use super::{AddAuthorityInstruction, AuthorityConfig, AuthorityType, build_client_action};
use crate::prelude::*;

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
    /// Authority type: "ed25519" (default) or "ed25519_session"
    #[serde(default = "default_authority_type")]
    pub authority_type: String,
    /// For ed25519_session: initial session key pubkey (defaults to new_authority if omitted)
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub initial_session_key: Option<Pubkey>,
    /// For ed25519_session: max session duration in slots (default ~4 days)
    #[serde(default = "default_max_session_length")]
    pub max_session_length: u64,
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

fn default_permission() -> String {
    "all".to_string()
}

fn default_authority_type() -> String {
    "ed25519".to_string()
}

fn default_max_session_length() -> u64 {
    864_000 // ~4 days at 400ms/slot
}

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

    // Build authority type and raw bytes based on authority_type field
    let (authority_type, authority_bytes): (AuthorityType, Vec<u8>) =
        match input.authority_type.as_str() {
            "ed25519_session" => {
                // CreateEd25519SessionAuthority layout: public_key[32] + session_key[32] + max_session_length[8]
                let session_key = input.initial_session_key.unwrap_or(input.new_authority);
                let mut bytes = Vec::with_capacity(72);
                bytes.extend_from_slice(input.new_authority.as_ref());
                bytes.extend_from_slice(session_key.as_ref());
                bytes.extend_from_slice(&input.max_session_length.to_le_bytes());
                (AuthorityType::Ed25519Session, bytes)
            }
            _ => (
                AuthorityType::Ed25519,
                input.new_authority.as_ref().to_vec(),
            ),
        };

    let ix = AddAuthorityInstruction::new_with_ed25519_authority(
        input.swig_account,
        input.fee_payer.pubkey(),
        input.acting_authority.pubkey(),
        input.acting_role_id,
        AuthorityConfig {
            authority_type,
            authority: &authority_bytes,
        },
        actions,
    )
    .map_err(|e| CommandError::msg(e.to_string()))?;

    let instruction = ix;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.acting_authority].into(),
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
        let new_authority = Keypair::new().pubkey();

        let ix = AddAuthorityInstruction::new_with_ed25519_authority(
            swig_account,
            kp.pubkey(),
            kp.pubkey(),
            0,
            AuthorityConfig {
                authority_type: AuthorityType::Ed25519,
                authority: new_authority.as_ref(),
            },
            build_client_action("all", None, None, None, None),
        )
        .unwrap();

        let instruction = ix;
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert!(!instruction.data.is_empty());
    }

    #[test]
    fn test_instruction_builder_sol_limit() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let new_authority = Keypair::new().pubkey();

        let ix = AddAuthorityInstruction::new_with_ed25519_authority(
            swig_account,
            kp.pubkey(),
            kp.pubkey(),
            0,
            AuthorityConfig {
                authority_type: AuthorityType::Ed25519,
                authority: new_authority.as_ref(),
            },
            build_client_action("sol_limit", Some(1_000_000_000), None, None, None),
        )
        .unwrap();

        let instruction = ix;
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
