use super::{helper, pda};
use crate::prelude::*;
use borsh::BorshSerialize;
use solana_program::instruction::AccountMeta;

const NAME: &str = "update_global_config";
const DEFINITION: &str = flow_lib::node_definition!("privacy_cash/update_global_config.jsonc");

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
    pub authority: Wallet,
    #[serde(default)]
    pub deposit_fee_rate: Option<u16>,
    #[serde(default)]
    pub withdrawal_fee_rate: Option<u16>,
    #[serde(default)]
    pub fee_error_margin: Option<u16>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (global_config, _) = pda::find_global_config();

    tracing::info!(
        "update_global_config: authority={}, deposit_fee={:?}, withdrawal_fee={:?}, error_margin={:?}",
        input.authority.pubkey(),
        input.deposit_fee_rate,
        input.withdrawal_fee_rate,
        input.fee_error_margin
    );

    // Accounts: UpdateGlobalConfig context
    let accounts = vec![
        AccountMeta::new(global_config, false), // global_config (mut, PDA)
        AccountMeta::new_readonly(input.authority.pubkey(), true), // authority (signer)
    ];

    // Anchor Option<u16> serialization: 0u8 for None, 1u8 + value for Some
    let mut args_data = Vec::new();
    BorshSerialize::serialize(&input.deposit_fee_rate, &mut args_data)?;
    BorshSerialize::serialize(&input.withdrawal_fee_rate, &mut args_data)?;
    BorshSerialize::serialize(&input.fee_error_margin, &mut args_data)?;

    let instruction = helper::build_instruction("update_global_config", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.authority.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;
    use solana_program::instruction::AccountMeta;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_all_some() {
        let authority: Pubkey = "97rSMQUukMDjA7PYErccyx7ZxbHvSDaeXp2ig5BwSrTf"
            .parse()
            .unwrap();
        let (global_config, _) = pda::find_global_config();

        let accounts = vec![
            AccountMeta::new(global_config, false),
            AccountMeta::new_readonly(authority, true),
        ];

        let mut args_data = Vec::new();
        BorshSerialize::serialize(&Some(25u16), &mut args_data).unwrap(); // deposit_fee_rate
        BorshSerialize::serialize(&Some(50u16), &mut args_data).unwrap(); // withdrawal_fee_rate
        BorshSerialize::serialize(&Some(500u16), &mut args_data).unwrap(); // fee_error_margin

        let ix = helper::build_instruction("update_global_config", accounts, args_data);

        assert_eq!(ix.program_id, pda::program_id());
        assert_eq!(ix.accounts.len(), 2);
        // 8 (disc) + 3 * (1 + 2) = 8 + 9 = 17 bytes
        assert_eq!(
            ix.data.len(),
            17,
            "all Some: disc(8) + 3 * Option<u16>::Some(3)"
        );
    }

    #[test]
    fn test_instruction_all_none() {
        let authority: Pubkey = "97rSMQUukMDjA7PYErccyx7ZxbHvSDaeXp2ig5BwSrTf"
            .parse()
            .unwrap();
        let (global_config, _) = pda::find_global_config();

        let accounts = vec![
            AccountMeta::new(global_config, false),
            AccountMeta::new_readonly(authority, true),
        ];

        let mut args_data = Vec::new();
        BorshSerialize::serialize(&None::<u16>, &mut args_data).unwrap();
        BorshSerialize::serialize(&None::<u16>, &mut args_data).unwrap();
        BorshSerialize::serialize(&None::<u16>, &mut args_data).unwrap();

        let ix = helper::build_instruction("update_global_config", accounts, args_data);

        // 8 (disc) + 3 * 1 = 11 bytes (each None is just 0u8)
        assert_eq!(
            ix.data.len(),
            11,
            "all None: disc(8) + 3 * Option<u16>::None(1)"
        );
    }

    #[test]
    fn test_option_u16_borsh_sizes() {
        let mut none_data = Vec::new();
        BorshSerialize::serialize(&None::<u16>, &mut none_data).unwrap();
        assert_eq!(none_data.len(), 1, "None<u16> = 1 byte (0x00)");
        assert_eq!(none_data[0], 0);

        let mut some_data = Vec::new();
        BorshSerialize::serialize(&Some(100u16), &mut some_data).unwrap();
        assert_eq!(some_data.len(), 3, "Some<u16> = 1 byte tag + 2 byte value");
        assert_eq!(some_data[0], 1);
    }

    #[tokio::test]
    #[ignore = "requires devnet admin key and funded wallet"]
    async fn test_devnet_update_global_config() {
        let ctx = CommandContext::default();
        let keypair = solana_keypair::Keypair::new();
        let wallet: Wallet = keypair.into();

        let output = run(
            ctx,
            Input {
                fee_payer: wallet.clone(),
                authority: wallet,
                deposit_fee_rate: Some(25),
                withdrawal_fee_rate: Some(50),
                fee_error_margin: Some(500),
                submit: false,
            },
        )
        .await
        .unwrap();

        assert!(output.signature.is_none());
    }
}
