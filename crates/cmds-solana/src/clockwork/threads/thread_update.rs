use crate::prelude::*;

use clockwork_client::thread::instruction::thread_update;

use clockwork_thread_program::state::ThreadSettings as ClockWorkThreadSettings;
use solana_sdk::pubkey::Pubkey;

use super::{Instruction, ThreadSettings, Trigger};

// Command Name
const THREAD_UPDATE: &str = "thread_update";

#[derive(Debug)]
pub struct ThreadUpdate;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub thread_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub thread: Pubkey,
    pub instructions: Option<Vec<Instruction>>,
    pub fee: Option<u64>,
    pub name: Option<String>,
    pub trigger: Option<Trigger>,
    pub rate_limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::signature")]
    signature: Signature,
}

#[async_trait]
impl CommandTrait for ThreadUpdate {
    fn name(&self) -> Name {
        THREAD_UPDATE.into()
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
            fee,
            name,
            trigger,
            rate_limit,
            instructions,
        } = value::from_map(inputs.clone())?;

        // FIXME
        let minimum_balance_for_rent_exemption = ctx
            .solana_client
            .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
                clockwork_thread_program::accounts::ThreadPause,
            >())
            .await?;

        let settings = ThreadSettings {
            fee,
            instructions,
            name,
            rate_limit,
            trigger,
        };

        let settings = ClockWorkThreadSettings::from(settings);

        // Create Instructions
        let instructions = vec![thread_update(thread_authority.pubkey(), thread, settings)];

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

inventory::submit!(CommandDescription::new(THREAD_UPDATE, |_| {
    Ok(Box::new(ThreadUpdate))
}));
