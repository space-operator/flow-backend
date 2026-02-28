use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{SWIG_PROGRAM_ID, SYSTEM_PROGRAM_ID, find_wallet_address};

const NAME: &str = "swig_migrate_wallet_address";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_migrate_wallet_address.jsonc");

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
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub wallet_address: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (wallet_address, wallet_bump) = find_wallet_address(&input.swig_account);

    // MigrateToWalletAddressV1Args (8 bytes):
    // discriminator(u16=12) + wallet_address_bump(u8) + padding(5)
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&12u16.to_le_bytes());  // discriminator = 12
    data.push(wallet_bump);                         // wallet_address_bump
    data.extend_from_slice(&[0u8; 5]);             // padding

    let accounts = vec![
        AccountMeta::new(input.swig_account, false),
        AccountMeta::new(input.authority.pubkey(), true),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new(wallet_address, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    let instruction = Instruction {
        program_id: SWIG_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature, wallet_address })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_keypair::{Keypair, Signer};

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_data_layout() {
        let swig_account = Keypair::new().pubkey();
        let kp = Keypair::new();
        let (wallet_address, wallet_bump) = find_wallet_address(&swig_account);

        // Build MigrateToWalletAddressV1 instruction (same as run())
        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&12u16.to_le_bytes());
        data.push(wallet_bump);
        data.extend_from_slice(&[0u8; 5]);

        let accounts = vec![
            AccountMeta::new(swig_account, false),
            AccountMeta::new(kp.pubkey(), true),
            AccountMeta::new(kp.pubkey(), true),
            AccountMeta::new(wallet_address, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ];

        let instruction = Instruction {
            program_id: SWIG_PROGRAM_ID,
            accounts,
            data: data.clone(),
        };

        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 5);
        // Verify discriminator = 12 (MigrateToWalletAddress)
        assert_eq!(u16::from_le_bytes([data[0], data[1]]), 12);
        assert_eq!(data[2], wallet_bump);
    }

    #[test]
    fn test_wallet_address_pda() {
        let swig_account = Keypair::new().pubkey();
        let (w1, b1) = find_wallet_address(&swig_account);
        let (w2, b2) = find_wallet_address(&swig_account);
        assert_eq!(w1, w2);
        assert_eq!(b1, b2);
    }

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run_integration() {
        let wallet: Wallet = Keypair::new().into();
        let swig_account = Keypair::new().pubkey();

        let input = Input {
            fee_payer: wallet.clone(),
            swig_account,
            authority: wallet,
            submit: true,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
