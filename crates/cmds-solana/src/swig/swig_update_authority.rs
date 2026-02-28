use crate::prelude::*;
use super::{
    to_pubkey_v2, to_instruction_v3, build_client_action,
    UpdateAuthorityInstruction, UpdateAuthorityData,
};

const NAME: &str = "swig_update_authority";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_update_authority.jsonc");

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
    pub authority_to_update_id: u32,
    #[serde(default = "default_operation")]
    pub operation: String,
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

fn default_operation() -> String { "replace_all".to_string() }
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

    let update_data = match input.operation.as_str() {
        "replace_all" => UpdateAuthorityData::ReplaceAll(actions),
        "add_actions" => UpdateAuthorityData::AddActions(actions),
        _ => UpdateAuthorityData::ReplaceAll(actions),
    };

    let ix_v2 = UpdateAuthorityInstruction::new_with_ed25519_authority(
        to_pubkey_v2(&input.swig_account),
        to_pubkey_v2(&input.fee_payer.pubkey()),
        to_pubkey_v2(&input.acting_authority.pubkey()),
        input.acting_role_id,
        input.authority_to_update_id,
        update_data,
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
    fn test_instruction_builder_replace_all() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let actions = build_client_action("all", None, None, None, None);

        let ix = UpdateAuthorityInstruction::new_with_ed25519_authority(
            to_pubkey_v2(&swig_account),
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&kp.pubkey()),
            0, 1,
            UpdateAuthorityData::ReplaceAll(actions),
        ).unwrap();

        let instruction = to_instruction_v3(ix);
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert!(!instruction.data.is_empty());
    }

    #[test]
    fn test_instruction_builder_add_actions() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let token_mint = Keypair::new().pubkey();
        let actions = build_client_action("token_limit", None, Some(&token_mint), Some(1_000_000), None);

        let ix = UpdateAuthorityInstruction::new_with_ed25519_authority(
            to_pubkey_v2(&swig_account),
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&kp.pubkey()),
            0, 1,
            UpdateAuthorityData::AddActions(actions),
        ).unwrap();

        let instruction = to_instruction_v3(ix);
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
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
            authority_to_update_id: 1,
            operation: "replace_all".to_string(),
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
