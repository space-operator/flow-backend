use crate::prelude::*;
use super::{
    to_pubkey_v2, to_instruction_v3, build_client_action,
    find_swig_pda, find_wallet_address,
    CreateInstruction, AuthorityConfig, AuthorityType,
};

const NAME: &str = "swig_create";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_create.jsonc");

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
    pub swig_id: [u8; 32],
    #[serde_as(as = "AsPubkey")]
    pub authority: Pubkey,
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

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub swig_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub wallet_address: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (swig_account, swig_bump) = find_swig_pda(&input.swig_id);
    let (wallet_address, wallet_bump) = find_wallet_address(&swig_account);

    let actions = build_client_action(
        &input.permission_type,
        input.sol_limit_amount,
        input.token_mint.as_ref(),
        input.token_limit_amount,
        input.program_id_permission.as_ref(),
    );

    let authority_v2 = to_pubkey_v2(&input.authority);
    let ix_v2 = CreateInstruction::new(
        to_pubkey_v2(&swig_account),
        swig_bump,
        to_pubkey_v2(&input.fee_payer.pubkey()),
        to_pubkey_v2(&wallet_address),
        wallet_bump,
        AuthorityConfig {
            authority_type: AuthorityType::Ed25519,
            authority: authority_v2.as_ref(),
        },
        actions,
        input.swig_id,
    ).map_err(|e| CommandError::msg(e.to_string()))?;

    let instruction = to_instruction_v3(ix_v2);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature, swig_account, wallet_address })
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
        let swig_id = [42u8; 32];
        let (swig_account, swig_bump) = find_swig_pda(&swig_id);
        let (wallet_address, wallet_bump) = find_wallet_address(&swig_account);

        let authority_v2 = to_pubkey_v2(&kp.pubkey());
        let ix = CreateInstruction::new(
            to_pubkey_v2(&swig_account),
            swig_bump,
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&wallet_address),
            wallet_bump,
            AuthorityConfig {
                authority_type: AuthorityType::Ed25519,
                authority: authority_v2.as_ref(),
            },
            build_client_action("all", None, None, None, None),
            swig_id,
        ).unwrap();

        let instruction = to_instruction_v3(ix);
        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert!(!instruction.data.is_empty());
    }

    #[test]
    fn test_pda_derivation() {
        let swig_id = [42u8; 32];
        let (swig1, bump1) = find_swig_pda(&swig_id);
        let (swig2, bump2) = find_swig_pda(&swig_id);
        assert_eq!(swig1, swig2);
        assert_eq!(bump1, bump2);

        let (wallet1, _) = find_wallet_address(&swig1);
        let (wallet2, _) = find_wallet_address(&swig1);
        assert_eq!(wallet1, wallet2);
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run_integration() {
        let wallet: Wallet = Keypair::new().into();
        let authority = Keypair::new().pubkey();
        let swig_id = [42u8; 32];

        let input = Input {
            fee_payer: wallet,
            swig_id,
            authority,
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
