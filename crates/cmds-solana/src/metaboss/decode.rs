use crate::prelude::*;
use metaboss_utils::commands::decode::decode;
use mpl_token_metadata::state::Metadata;

#[derive(Clone, Debug)]
pub struct Decode;

const DECODE: &str = "decode";

// Inputs
const MINT_ACCOUNT: &str = "mint_account";

// Outputs
const METADATA: &str = "metadata";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub metadata: Metadata,
}

#[async_trait]
impl CommandTrait for Decode {
    fn name(&self) -> Name {
        DECODE.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [CmdInput {
            name: MINT_ACCOUNT.into(),
            type_bounds: [ValueType::Pubkey].to_vec(),
            required: true,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: METADATA.into(),
            r#type: ValueType::Json,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input: Input = value::from_map(inputs)?;

        let metadata = decode(&ctx.solana_client, &input.mint_account)
            .await
            .map_err(crate::Error::custom)?;

        Ok(value::to_map(&Output { metadata })?)
    }
}

inventory::submit!(CommandDescription::new(DECODE, |_| Ok(Box::new(Decode))));
