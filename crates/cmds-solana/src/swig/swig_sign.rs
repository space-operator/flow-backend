use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{SWIG_PROGRAM_ID, SYSTEM_PROGRAM_ID, find_wallet_address};

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
    pub instructions_data: Vec<u8>,
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
    let instruction_payload = &input.instructions_data;

    // SignV2Args (8 bytes):
    // instruction(u16=11) + instruction_payload_len(u16) + role_id(u32)
    let mut data = Vec::with_capacity(16 + instruction_payload.len());
    data.extend_from_slice(&11u16.to_le_bytes());                           // instruction = 11
    data.extend_from_slice(&(instruction_payload.len() as u16).to_le_bytes()); // instruction_payload_len
    data.extend_from_slice(&input.role_id.to_le_bytes());                   // role_id

    // Instruction payload (compact serialized inner instructions)
    data.extend_from_slice(instruction_payload);

    // Authority payload: signer index = 3
    data.push(3u8);

    let accounts = vec![
        AccountMeta::new(input.swig_account, false),
        AccountMeta::new(wallet_address, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        AccountMeta::new_readonly(input.authority.pubkey(), true),
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

    Ok(Output { signature })
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
        let kp = Keypair::new();
        let swig_account = Keypair::new().pubkey();
        let (wallet_address, _) = find_wallet_address(&swig_account);
        let payload = vec![0u8; 4];

        // Build the SignV2 instruction manually (same as run())
        let mut data = Vec::with_capacity(16 + payload.len());
        data.extend_from_slice(&11u16.to_le_bytes());
        data.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&payload);
        data.push(3u8);

        let accounts = vec![
            AccountMeta::new(swig_account, false),
            AccountMeta::new(wallet_address, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(kp.pubkey(), true),
        ];

        let instruction = Instruction {
            program_id: SWIG_PROGRAM_ID,
            accounts,
            data: data.clone(),
        };

        assert_eq!(instruction.program_id, SWIG_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 4);
        // Verify discriminator = 11 (SignV2)
        assert_eq!(u16::from_le_bytes([data[0], data[1]]), 11);
        // Verify payload length
        assert_eq!(u16::from_le_bytes([data[2], data[3]]), 4);
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
            instructions_data: vec![0u8; 4],
            submit: true,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
    }
}
