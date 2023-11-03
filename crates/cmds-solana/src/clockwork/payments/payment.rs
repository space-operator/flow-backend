use crate::prelude::*;
use anchor_lang_26::{solana_program::sysvar, InstructionData, ToAccountMetas};
use anchor_spl_26::{associated_token, token};
use clockwork_client::thread::{
    instruction::thread_create,
    state::{Thread, Trigger},
};

use clockwork_utils::thread::SerializableInstruction as ClockWorkInstruction;
use payments::state::Payment as ClockworkPayment;
use solana_program::{instruction::Instruction, system_program};
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;

const CREATE_PAYMENT: &str = "create_payment";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/clockwork/payments/create_payment.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(CREATE_PAYMENT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(CREATE_PAYMENT, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TriggerInput {
    IsImmediate {
        is_immediate: bool,
    },
    Schedule {
        schedule: String,
        is_skippable: bool,
    },
    MonitorAccount {
        #[serde(with = "value::pubkey")]
        monitor_account: Pubkey,
        offset: u64,
        size: u64,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub token_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_mint: Pubkey,
    #[serde(with = "value::pubkey")]
    pub recipient: Pubkey,
    pub amount: u64,
    pub trigger: TriggerInput,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let trigger = match input.trigger {
        TriggerInput::IsImmediate { is_immediate: _ } => Trigger::Now,
        TriggerInput::Schedule {
            schedule,
            is_skippable,
        } => Trigger::Cron {
            schedule,
            skippable: is_skippable,
        },
        TriggerInput::MonitorAccount {
            monitor_account,
            offset,
            size,
        } => Trigger::Account {
            address: monitor_account,
            offset,
            size,
        },
    };

    // Thread Authority is the Payer
    let thread_authority = input.payer.pubkey();

    // Derive PDAs
    let payment = ClockworkPayment::pubkey(input.payer.pubkey(), input.token_mint, input.recipient);
    let thread = Thread::pubkey(thread_authority, "payment".into());
    let recipient_ata_pubkey = get_associated_token_address(&input.recipient, &input.token_mint);

    // FIXME - check size
    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(816 + 608 + 512)
        .await?;

    // Create Payment Instruction
    let accounts = payments::accounts::CreatePayment {
        associated_token_program: associated_token::ID,
        authority: input.payer.pubkey(),
        authority_token_account: input.token_account,
        mint: input.token_mint,
        payment,
        recipient: input.recipient,
        rent: sysvar::rent::ID,
        system_program: system_program::ID,
        token_program: token::ID,
    }
    .to_account_metas(None);

    let data = payments::instruction::CreatePayment {
        amount: input.amount,
    }
    .data();

    let payment_instruction = Instruction {
        program_id: payments::ID,
        accounts,
        data,
    };

    // Create Disbursement Instruction
    let accounts = payments::accounts::DisbursePayment {
        associated_token_program: associated_token::ID,
        authority: input.payer.pubkey(),
        authority_token_account: input.token_account,
        mint: input.token_mint,
        payer: input.payer.pubkey(),
        payment,
        thread,
        recipient: input.recipient,
        recipient_token_account: recipient_ata_pubkey,
        rent: sysvar::rent::ID,
        system_program: system_program::ID,
        token_program: token::ID,
    }
    .to_account_metas(None);

    let distribute_payment_ix: ClockWorkInstruction = Instruction {
        program_id: payments::ID,
        accounts,
        data: payments::instruction::DisbursePayment.data(),
    }
    .into();

    // Create Thread Instruction with Disbursement as the first instruction
    let thread_create_instruction = thread_create(
        input.amount,
        input.payer.pubkey(),
        "payment".into(),
        vec![distribute_payment_ix],
        input.payer.pubkey(),
        thread,
        trigger,
    );

    // Bundle it all up
    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair()].into(),
        instructions: [payment_instruction, thread_create_instruction].into(),
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
