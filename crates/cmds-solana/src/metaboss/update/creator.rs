use crate::prelude::*;
use metaboss_utils::commands::update::update_creator;

#[derive(Clone, Debug)]
pub struct UpdateCreators;

const UPDATE_CREATORS: &str = "update_creator";

// Inputs
const KEYPAIR: &str = "keypair";
const MINT_ACCOUNT: &str = "mint_account";
const CREATOR: &str = "creator";
const APPEND: &str = "append";

// Outputs
const SIGNATURE: &str = "signature";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub keypair: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub creator: Pubkey,
    pub append: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::signature")]
    pub signature: Signature,
}

#[async_trait]
impl CommandTrait for UpdateCreators {
    fn name(&self) -> Name {
        UPDATE_CREATORS.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: KEYPAIR.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: MINT_ACCOUNT.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: CREATOR.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: APPEND.into(),
                type_bounds: [ValueType::Bool].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: SIGNATURE.into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input: Input = value::from_map(inputs)?;

        let mut tx = update_creator(
            &ctx.solana_client,
            input.keypair.clone_keypair(),
            input.mint_account,
            input.creator.to_string(),
            input.append,
        )
        .await
        .map_err(crate::Error::custom)?;

        let recent_blockhash = ctx.solana_client.get_latest_blockhash().await?;
        try_sign_wallet(&ctx, &mut tx, &[&input.keypair], recent_blockhash).await?;

        let sig = submit_transaction(&ctx.solana_client, tx).await?;

        Ok(value::to_map(&Output { signature: sig })?)
    }
}

inventory::submit!(CommandDescription::new(UPDATE_CREATORS, |_| Ok(Box::new(
    UpdateCreators
))));
