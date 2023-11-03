use crate::prelude::*;
use solana_program::instruction::Instruction;

#[derive(Debug, Clone)]
pub struct ApproveUseAuthority;

impl ApproveUseAuthority {
    #[allow(clippy::too_many_arguments)]
    async fn command_approve_use_authority(
        &self,
        rpc_client: &RpcClient,
        use_authority_record_pubkey: Pubkey,
        user: Pubkey,
        owner: Pubkey,
        payer: Pubkey,
        token_account: Pubkey,
        metadata_pubkey: Pubkey,
        mint: Pubkey,
        burner: Pubkey,
        number_of_uses: u64,
    ) -> crate::Result<(u64, Vec<Instruction>)> {
        let minimum_balance_for_rent_exemption = rpc_client
            .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
                mpl_token_metadata::state::UseAuthorityRecord,
            >())
            .await?;

        let instructions = vec![mpl_token_metadata::instruction::approve_use_authority(
            mpl_token_metadata::id(),
            use_authority_record_pubkey,
            user,
            owner,
            payer,
            token_account,
            metadata_pubkey,
            mint,
            burner,
            number_of_uses,
        )];

        Ok((minimum_balance_for_rent_exemption, instructions))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub use_authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub owner: Keypair,
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub token_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub burner: Pubkey,
    pub number_of_uses: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

const APPROVE_USE_AUTHORITY: &str = "approve_use_authority";

// Inputs
const USE_AUTHORITY: &str = "use_authority";
const OWNER: &str = "owner";
const FEE_PAYER: &str = "fee_payer";
const TOKEN_ACCOUNT: &str = "token_account";
const MINT_ACCOUNT: &str = "mint_account";
const BURNER: &str = "burner";
const NUMBER_OF_USES: &str = "number_of_uses";
const SUBMIT: &str = "submit";

// Outputs
const SIGNATURE: &str = "signature";

#[async_trait]
impl CommandTrait for ApproveUseAuthority {
    fn name(&self) -> Name {
        APPROVE_USE_AUTHORITY.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: USE_AUTHORITY.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: OWNER.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: FEE_PAYER.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: TOKEN_ACCOUNT.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
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
                name: BURNER.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: NUMBER_OF_USES.into(),
                type_bounds: [ValueType::U64].to_vec(),
                required: true,
                passthrough: false,
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
            use_authority,
            owner,
            fee_payer,
            token_account,
            mint_account,
            burner,
            number_of_uses,
            submit,
        } = value::from_map(inputs)?;

        let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&mint_account);

        let (use_authority_record_pubkey, _) =
            mpl_token_metadata::pda::find_use_authority_account(&mint_account, &use_authority);

        let (minimum_balance_for_rent_exemption, instructions) = self
            .command_approve_use_authority(
                &ctx.solana_client,
                use_authority_record_pubkey,
                use_authority,
                owner.pubkey(),
                fee_payer.pubkey(),
                token_account,
                metadata_account,
                mint_account,
                burner,
                number_of_uses,
            )
            .await?;

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
            &[&owner, &fee_payer],
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

inventory::submit!(CommandDescription::new(APPROVE_USE_AUTHORITY, |_| Ok(
    Box::new(ApproveUseAuthority)
)));
