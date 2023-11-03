use super::Trigger;
use crate::prelude::*;
use clockwork_client::thread::instruction::thread_create;
use clockwork_client::thread::state::Thread;
use clockwork_utils::thread::SerializableInstruction as ClockWorkInstruction;
use clockwork_utils::thread::Trigger as ClockWorkTrigger;
use solana_program::instruction::Instruction;

// Command Name
const THREAD_CREATE: &str = "thread_create";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/clockwork/threads/thread_create.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(THREAD_CREATE)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(THREAD_CREATE, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub amount: u64,
    #[serde(with = "value::keypair")]
    pub thread_authority: Keypair,
    pub id: String,
    pub instructions: Vec<Instruction>,
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    pub trigger: Trigger,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}
async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    // FIXME
    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            clockwork_thread_program::accounts::ThreadCreate,
        >())
        .await?;

    // Instruction to ClockWork SerializableInstruction
    let mut instruction_chain = vec![];
    for instruction in input.instructions {
        let instruction = ClockWorkInstruction::from(instruction);
        instruction_chain.push(instruction);
    }

    // Trigger to ClockWork Trigger
    let trigger = ClockWorkTrigger::from(input.trigger);

    let id = input.id.as_bytes().to_vec();

    let thread = Thread::pubkey(input.thread_authority.pubkey(), id.clone());

    // Create Instructions
    let instruction = thread_create(
        minimum_balance_for_rent_exemption + input.amount,
        input.thread_authority.pubkey(),
        id.clone(),
        instruction_chain,
        input.payer.pubkey(),
        thread,
        trigger,
    );

    // Bundle it all up
    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.thread_authority.clone_keypair(),
        ]
        .into(),
        instructions: [instruction].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "thread" => thread
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
