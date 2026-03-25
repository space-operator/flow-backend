use super::{WithdrawFromSubAccountInstruction, find_wallet_address};
use crate::prelude::*;

const NAME: &str = "swig_withdraw_from_sub_account";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_withdraw_from_sub_account.jsonc");

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
    pub authority: Wallet,
    pub role_id: u32,
    #[serde_as(as = "AsPubkey")]
    pub sub_account: Pubkey,
    pub amount: u64,
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

    let ix = WithdrawFromSubAccountInstruction::new_with_ed25519_authority(
        input.swig_account,
        input.authority.pubkey(),
        input.fee_payer.pubkey(),
        input.sub_account,
        wallet_address,
        input.role_id,
        input.amount,
    )
    .map_err(|e| CommandError::msg(e.to_string()))?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [ix].into(),
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
    fn test_instruction_builder() {
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let sub_account = Keypair::new().pubkey();
        let (wallet_address, _) = find_wallet_address(&swig_account);

        let ix = WithdrawFromSubAccountInstruction::new_with_ed25519_authority(
            swig_account,
            kp.pubkey(),
            kp.pubkey(),
            sub_account,
            wallet_address,
            0,
            1_000_000,
        )
        .unwrap();
        assert_eq!(ix.program_id, SWIG_PROGRAM_ID);
        assert!(!ix.data.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run_integration() {
        let wallet: Wallet = Keypair::new().into();
        let swig_account = Keypair::new().pubkey();
        let sub_account = Keypair::new().pubkey();
        let input = Input {
            fee_payer: wallet.clone(),
            swig_account,
            authority: wallet,
            role_id: 0,
            sub_account,
            amount: 1_000_000,
            submit: true,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
