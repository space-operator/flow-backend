use super::utils::find_proxy_authority_address;
use crate::prelude::*;
use anchor_lang::InstructionData;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    system_program,
};
use solana_sdk::pubkey::Pubkey;
use space_wrapper::instruction::CreateProxyAuthority as Proxy;

fn create_create_proxy_instruction(proxy_authority: &Pubkey, authority: &Pubkey) -> Instruction {
    let accounts = [
        AccountMeta::new(*proxy_authority, false),
        AccountMeta::new(*authority, true),
        AccountMeta::new(system_program::ID, false),
    ]
    .to_vec();

    Instruction {
        program_id: space_wrapper::ID,
        accounts,
        data: Proxy.data(),
    }
}

#[derive(Debug)]
pub struct CreateProxyAuthority;

impl CreateProxyAuthority {
    async fn command_create_proxy_authority(
        &self,
        rpc_client: &RpcClient,
        payer: Pubkey,
    ) -> crate::Result<(u64, Vec<Instruction>)> {
        // TODO: get size of proxy account from space-wrapper crate
        let min_rent = rpc_client
            .get_minimum_balance_for_rent_exemption(44)
            .await?;

        let proxy_authority = find_proxy_authority_address(&payer);

        let instruction = create_create_proxy_instruction(&proxy_authority, &payer);

        Ok((min_rent, [instruction].to_vec()))
    }
}

// Command Name
const CREATE_PROXY_AUTHORITY: &str = "create_proxy_authority";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub authority: Keypair,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    #[serde(with = "value::pubkey")]
    proxy_authority: Pubkey,
}

// Inputs
const AUTHORITY: &str = "authority";

// Outputs
const SIGNATURE: &str = "signature";
const PROXY_AUTHORITY: &str = "proxy_authority";

#[async_trait]
impl CommandTrait for CreateProxyAuthority {
    fn name(&self) -> Name {
        CREATE_PROXY_AUTHORITY.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [CmdInput {
            name: AUTHORITY.into(),
            type_bounds: [ValueType::Keypair].to_vec(),
            required: true,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [
            CmdOutput {
                name: SIGNATURE.into(),
                r#type: ValueType::String,
            },
            CmdOutput {
                name: PROXY_AUTHORITY.into(),
                r#type: ValueType::Pubkey,
            },
        ]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input { authority } = value::from_map(inputs)?;

        let proxy_authority = find_proxy_authority_address(&authority.pubkey());

        let (minimum_balance_for_rent_exemption, instructions) = self
            .command_create_proxy_authority(&ctx.solana_client, authority.pubkey())
            .await?;

        let (mut transaction, recent_blockhash) = execute(
            &ctx.solana_client,
            &authority.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )
        .await?;

        try_sign_wallet(&ctx, &mut transaction, &[&authority], recent_blockhash).await?;

        let signature = Some(submit_transaction(&ctx.solana_client, transaction).await?);

        Ok(value::to_map(&Output {
            signature,
            proxy_authority,
        })?)
    }
}

inventory::submit!(CommandDescription::new(CREATE_PROXY_AUTHORITY, |_| {
    Ok(Box::new(CreateProxyAuthority))
}));
