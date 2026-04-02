use crate::prelude::*;
use spl_token_2022::extension::ExtensionType;
use spl_token_2022::state::Mint;
use spl_token_2022_interface::extension::ExtensionType as InterfaceExtensionType;

const NAME: &str = "get_account_data_size";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/get_account_data_size.jsonc");

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
    pub extension_types: Vec<InterfaceExtensionType>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    pub size: u64,
    pub lamports: u64,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let extension_types: Vec<ExtensionType> = input
        .extension_types
        .iter()
        .map(|et| {
            let discriminant: u16 = (*et).into();
            ExtensionType::try_from(discriminant)
                .map_err(|_| CommandError::msg(format!("unknown extension type: {}", discriminant)))
        })
        .collect::<Result<_, _>>()?;

    let size = ExtensionType::try_calculate_account_len::<Mint>(&extension_types)?;

    let lamports = ctx
        .solana_client()
        .get_minimum_balance_for_rent_exemption(size)
        .await?;

    let size = size as u64;

    let ins = Instructions::default();
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        size,
        lamports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_calculate_size() {
        let size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
                .unwrap();
        // Base Mint (82) + account type (1) + extension header + TransferFeeConfig
        assert!(size > 82);
    }
}
