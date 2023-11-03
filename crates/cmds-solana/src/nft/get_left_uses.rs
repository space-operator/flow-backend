use crate::prelude::*;
use async_trait::async_trait;
use mpl_token_metadata::state::Metadata;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct GetLeftUses;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub left_uses: Option<u64>,
}

const GET_LEFT_USES: &str = "get_left_uses";

// Inputs
const MINT_ACCOUNT: &str = "mint_account";

// Outputs
const LEFT_USES: &str = "left_uses";

#[async_trait]
impl CommandTrait for GetLeftUses {
    fn name(&self) -> Name {
        GET_LEFT_USES.into()
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
            name: LEFT_USES.into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input { mint_account } = value::from_map(inputs)?;

        let (metadata_account, _) = Metadata::find_pda(&mint_account);

        let account_data = ctx
            .solana_client
            .get_account_data(&metadata_account)
            .await?;

        let mut account_data_ptr = account_data.as_slice();

        let metadata = <Metadata as borsh::BorshDeserialize>::deserialize(&mut account_data_ptr)?;

        let left_uses = metadata.uses.map(|v| v.remaining);

        Ok(value::to_map(&Output { left_uses })?)
    }
}

inventory::submit!(CommandDescription::new(GET_LEFT_USES, |_| Ok(Box::new(
    GetLeftUses
))));
