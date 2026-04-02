use super::{SWIG_PROGRAM_ID, SYSTEM_PROGRAM_ID, find_wallet_address};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use swig_interface::compact_instructions;

const NAME: &str = "swig_sign";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_sign.jsonc");

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
    #[serde(default)]
    pub role_id: u32,
    pub instructions: Vec<Instruction>,
    #[serde(default)]
    pub signers: Vec<Wallet>,
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

    // Base accounts: [swig_account, wallet_address, system_program, authority]
    // Authority is at index 3 → signer_index byte = 3
    let base_accounts = vec![
        AccountMeta::new(input.swig_account, false),
        AccountMeta::new(wallet_address, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
    ];

    // compact_instructions deduplicates accounts and converts pubkeys to indexes
    let (mut accounts, compact_ixs) =
        compact_instructions(input.swig_account, base_accounts, input.instructions);

    // Mark any additional signer wallets that appear in the accounts list
    let signer_pubkeys: Vec<Pubkey> = input.signers.iter().map(|w| w.pubkey()).collect();
    for account in &mut accounts {
        if signer_pubkeys.contains(&account.pubkey) {
            account.is_signer = true;
        }
    }

    let ix_bytes = compact_ixs.into_bytes();

    // SignV2Args layout (8 bytes):
    // instruction(u16=11) + instruction_payload_len(u16) + role_id(u32)
    let mut data = Vec::with_capacity(8 + ix_bytes.len() + 1);
    data.extend_from_slice(&11u16.to_le_bytes());
    data.extend_from_slice(&(ix_bytes.len() as u16).to_le_bytes());
    data.extend_from_slice(&input.role_id.to_le_bytes());
    data.extend_from_slice(&ix_bytes);
    data.push(3u8); // authority signer index

    let instruction = Instruction {
        program_id: SWIG_PROGRAM_ID,
        accounts,
        data,
    };

    let mut all_signers = vec![input.fee_payer.clone(), input.authority];
    all_signers.extend(input.signers);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: all_signers.into(),
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
    use solana_keypair::Keypair;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_compact_instruction_layout() {
        let authority = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let (wallet_address, _) = find_wallet_address(&swig_account);
        let recipient = Keypair::new().pubkey();

        // A minimal inner instruction (e.g. SOL transfer) built directly
        let inner_ix = Instruction {
            program_id: SYSTEM_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(wallet_address, true),
                AccountMeta::new(recipient, false),
            ],
            data: {
                // system transfer discriminator (u32=2) + lamports (u64)
                let mut d = 2u32.to_le_bytes().to_vec();
                d.extend_from_slice(&1_000_000u64.to_le_bytes());
                d
            },
        };

        let base_accounts = vec![
            AccountMeta::new(swig_account, false),
            AccountMeta::new(wallet_address, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
        ];

        let (accounts, compact_ixs) =
            compact_instructions(swig_account, base_accounts, vec![inner_ix]);
        let ix_bytes = compact_ixs.into_bytes();

        let mut data = Vec::with_capacity(8 + ix_bytes.len() + 1);
        data.extend_from_slice(&11u16.to_le_bytes());
        data.extend_from_slice(&(ix_bytes.len() as u16).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&ix_bytes);
        data.push(3u8);

        // Verify discriminator = 11 (SignV2)
        assert_eq!(u16::from_le_bytes([data[0], data[1]]), 11);
        // Verify payload_len matches actual ix_bytes length
        assert_eq!(
            u16::from_le_bytes([data[2], data[3]]) as usize,
            ix_bytes.len()
        );
        // Base accounts (4) + inner ix accounts deduped
        assert!(accounts.len() >= 4);
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
            role_id: 0,
            instructions: vec![],
            signers: vec![],
            submit: true,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
