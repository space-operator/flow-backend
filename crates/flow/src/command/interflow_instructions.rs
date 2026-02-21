use super::interflow::get_interflow_id;
use crate::{
    command::prelude::*,
    flow_graph::FlowRunResult,
    flow_registry::{StartFlowOptions, start_flow},
};
use bytes::Bytes;
use flow_lib::solana::Instructions;
use tower::{Service, ServiceExt};

pub const INTERFLOW_INSTRUCTIONS: &str = "interflow_instructions";

struct Interflow {
    id: FlowId,
    inputs: Vec<Input>,
}

impl Interflow {
    fn new(n: &NodeData) -> Result<Self, CommandError> {
        let id = get_interflow_id(n)?;
        let inputs = n
            .inputs
            .iter()
            .map(|x| Input {
                name: x.name.clone(),
                type_bounds: [ValueType::Free].to_vec(),
                required: x.required,
                passthrough: x.passthrough,
            })
            .collect();

        Ok(Self { id, inputs })
    }
}

fn build_error(r: &FlowRunResult) -> CommandError {
    let mut msg = "no instructions\n".to_owned();
    for e in &r.flow_errors {
        msg.push_str(e);
        msg.push('\n');
    }
    for e in r.node_errors.values().flatten() {
        msg.push_str(e);
        msg.push('\n');
    }

    CommandError::msg(msg)
}

#[async_trait(?Send)]
impl CommandTrait for Interflow {
    fn name(&self) -> Name {
        INTERFLOW_INSTRUCTIONS.into()
    }

    fn inputs(&self) -> Vec<Input> {
        self.inputs.clone()
    }

    fn outputs(&self) -> Vec<Output> {
        [
            Output {
                name: "fee_payer".into(),
                r#type: ValueType::Pubkey,
                optional: false,
            },
            Output {
                name: "signers".into(),
                r#type: ValueType::Array,
                optional: false,
            },
            Output {
                name: "instructions".into(),
                r#type: ValueType::Array,
                optional: false,
            },
        ]
        .into()
    }

    async fn run(&self, ctx: CommandContext, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let start = ctx
            .get::<start_flow::Svc>()
            .ok_or_else(|| anyhow::anyhow!("start_flow::Svc not found"))?
            .clone();

        let (_, handle) = start
            .ready_oneshot()
            .await?
            .call(start_flow::Request {
                flow_id: self.id,
                inputs,
                options: StartFlowOptions {
                    collect_instructions: true,
                    origin: ctx.new_interflow_origin(),
                    solana_client: Some(ctx.solana_config().clone()),
                    ..Default::default()
                },
            })
            .await?;
        let result = handle.await?;

        if result.instructions.is_none() {
            return Err(build_error(&result));
        }
        let ins = result.instructions.unwrap();
        Ok(instruction_to_output(ins)?)
    }
}

pub(crate) fn instruction_to_output(ins: Instructions) -> Result<value::Map, value::Error> {
    let signers = ins
        .signers
        .into_iter()
        .map(|w| value::to_value(&w))
        .collect::<Result<Vec<_>, _>>()?;
    let instructions = ins
        .instructions
        .into_iter()
        .map(|i| {
            Value::Map(value::map! {
                "program_id" => i.program_id,
                "accounts" => i.accounts.into_iter().map(|a| Value::Map(value::map! {
                    "pubkey" => a.pubkey,
                    "is_signer" => a.is_signer,
                    "is_writable" => a.is_writable,
                })).collect::<Vec<_>>(),
                "data" => Bytes::from(i.data),
            })
        })
        .collect::<Vec<_>>();
    Ok(value::map! {
        "fee_payer" => ins.fee_payer,
        "signers" => signers,
        "instructions" => instructions,
    })
}

flow_lib::submit!(CommandDescription::new(
    INTERFLOW_INSTRUCTIONS,
    |data: &NodeData| { Ok(Box::new(Interflow::new(data)?)) }
));
