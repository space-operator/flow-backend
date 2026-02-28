use crate::prelude::*;
use super::{to_pubkey_v2, to_instruction_v3, find_wallet_address, CloseTokenAccountV1Instruction};

const NAME: &str = "swig_close_token_account";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_close_token_account.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub destination: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
    pub authority: Wallet,
    #[serde(default)]
    pub role_id: u32,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (wallet_address, _) = find_wallet_address(&input.swig_account);

    let ix_v2 = CloseTokenAccountV1Instruction::new_with_ed25519_authority(
        to_pubkey_v2(&input.swig_account),
        to_pubkey_v2(&wallet_address),
        to_pubkey_v2(&input.authority.pubkey()),
        to_pubkey_v2(&input.destination),
        to_pubkey_v2(&input.token_program),
        vec![to_pubkey_v2(&input.token_account)],
        input.role_id,
    ).map_err(|e| CommandError::msg(e.to_string()))?;

    let instruction = to_instruction_v3(ix_v2);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
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
        let (wallet_address, _) = find_wallet_address(&swig_account);
        let destination = Keypair::new().pubkey();
        let token_account = Keypair::new().pubkey();
        let token_program: Pubkey = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap();

        let ix = CloseTokenAccountV1Instruction::new_with_ed25519_authority(
            to_pubkey_v2(&swig_account),
            to_pubkey_v2(&wallet_address),
            to_pubkey_v2(&kp.pubkey()),
            to_pubkey_v2(&destination),
            to_pubkey_v2(&token_program),
            vec![to_pubkey_v2(&token_account)],
            0,
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
        let destination = Keypair::new().pubkey();
        let token_account = Keypair::new().pubkey();
        let token_program: Pubkey = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap();

        let input = Input {
            fee_payer: wallet.clone(),
            swig_account,
            destination,
            token_account,
            token_program,
            authority: wallet,
            role_id: 0,
            submit: true,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
