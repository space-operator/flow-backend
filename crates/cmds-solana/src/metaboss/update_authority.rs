use crate::prelude::*;
use metaboss_utils::commands::update::{set_update_authority, SetUpdateAuthorityArgs};

#[derive(Clone, Debug)]
pub struct UpdateAuthority;

const UPDATE_AUTHORITY: &str = "update_authority";

// Inputs
const KEYPAIR: &str = "keypair";
const PAYER: &str = "payer";
const MINT_ACCOUNT: &str = "mint_account";
const NEW_UPDATE_AUTHORITY: &str = "new_update_authority";

// Outputs
const SIGNATURE: &str = "signature";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub keypair: Keypair,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub new_update_authority: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::signature")]
    pub signature: Signature,
}

#[async_trait]
impl CommandTrait for UpdateAuthority {
    fn name(&self) -> Name {
        UPDATE_AUTHORITY.into()
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
                name: PAYER.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: MINT_ACCOUNT.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: NEW_UPDATE_AUTHORITY.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
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

        let args = SetUpdateAuthorityArgs {
            client: &ctx.solana_client,
            keypair: Arc::new(input.keypair.clone_keypair()),
            payer: Arc::new(input.payer),
            mint_account: input.mint_account,
            new_authority: input.new_update_authority,
        };

        let mut tx = set_update_authority(&args)
            .await
            .map_err(crate::Error::custom)?;

        let recent_blockhash = ctx.solana_client.get_latest_blockhash().await?;
        try_sign_wallet(&ctx, &mut tx, &[&input.keypair], recent_blockhash).await?;

        let sig = submit_transaction(&ctx.solana_client, tx).await?;

        Ok(value::to_map(&Output { signature: sig })?)
    }
}

inventory::submit!(CommandDescription::new(UPDATE_AUTHORITY, |_| Ok(Box::new(
    UpdateAuthority
))));
