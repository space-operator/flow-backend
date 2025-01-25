use crate::prelude::*;

use clockwork_client::thread::instruction::thread_pause;

use solana_sdk::pubkey::Pubkey;

// Command Name
const THREAD_PAUSE: &str = "thread_pause";

#[derive(Debug)]
pub struct ThreadPause;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub thread_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub thread: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::signature")]
    signature: Signature,
}

#[async_trait]
impl CommandTrait for ThreadPause {
    fn name(&self) -> Name {
        THREAD_PAUSE.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: "THREAD_AUTHORITY".into(),
                type_bounds: [ValueType::Keypair, ValueType::String].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: "THREAD".into(),
                type_bounds: [ValueType::Pubkey, ValueType::Keypair, ValueType::String].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: "SIGNATURE".into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input {
            thread_authority,
            thread,
        } = value::from_map(inputs.clone())?;

        // FIXME
        let minimum_balance_for_rent_exemption = ctx
            .solana_client
            .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
                clockwork_thread_program::accounts::ThreadPause,
            >())
            .await?;

        // Create Instructions
        let instructions = vec![thread_pause(thread_authority.pubkey(), thread)];

        //
        let (mut transaction, recent_blockhash) = execute(
            &ctx.solana_client,
            &thread_authority.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )
        .await?;

        try_sign_wallet(
            &ctx,
            &mut transaction,
            &[&thread_authority],
            recent_blockhash,
        )
        .await?;

        let signature = submit_transaction(&ctx.solana_client, transaction).await?;

        Ok(value::to_map(&Output { signature })?)
    }
}

flow_lib::submit!(CommandDescription::new(THREAD_PAUSE, |_| {
    Ok(Box::new(ThreadPause))
}));
