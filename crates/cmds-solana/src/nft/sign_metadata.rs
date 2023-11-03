use crate::prelude::*;
use solana_program::instruction::Instruction;

#[derive(Debug, Clone)]
pub struct SignMetadata;

impl SignMetadata {
    fn command_sign_metadata(
        &self,
        metadata: Pubkey,
        creator: Pubkey,
    ) -> crate::Result<(u64, Vec<Instruction>)> {
        let instructions = vec![mpl_token_metadata::instruction::sign_metadata(
            mpl_token_metadata::id(),
            metadata,
            creator,
        )];

        Ok((0, instructions))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    #[serde(with = "value::keypair")]
    creator: Keypair,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

const SIGN_METADATA: &str = "sign_metadata";

// Inputs
const FEE_PAYER: &str = "fee_payer";
const MINT_ACCOUNT: &str = "mint_account";
const CREATOR: &str = "creator";
const SUBMIT: &str = "submit";

// Outputs
const SIGNATURE: &str = "signature";

#[async_trait]
impl CommandTrait for SignMetadata {
    fn name(&self) -> Name {
        SIGN_METADATA.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: FEE_PAYER.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: MINT_ACCOUNT.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: CREATOR.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: SUBMIT.into(),
                type_bounds: [ValueType::Bool].to_vec(),
                required: false,
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
        let Input {
            fee_payer,
            mint_account,
            creator,
            submit,
        } = value::from_map(inputs)?;

        let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&mint_account);

        let (minimum_balance_for_rent_exemption, instructions) =
            self.command_sign_metadata(metadata_account, creator.pubkey())?;

        let (mut transaction, recent_blockhash) = execute(
            &ctx.solana_client,
            &fee_payer.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )
        .await?;

        try_sign_wallet(
            &ctx,
            &mut transaction,
            &[&creator, &fee_payer],
            recent_blockhash,
        )
        .await?;

        let signature = if submit {
            Some(submit_transaction(&ctx.solana_client, transaction).await?)
        } else {
            None
        };

        Ok(value::to_map(&Output { signature })?)
    }
}

inventory::submit!(CommandDescription::new(SIGN_METADATA, |_| Ok(Box::new(
    SignMetadata
))));
