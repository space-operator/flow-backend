use crate::{
    command::prelude::*,
    flow_registry::{StartFlowOptions, start_flow},
};
use anyhow::anyhow;
use flow_lib::command::InstructionInfo;
use tower::{Service, ServiceExt};

pub const INTERFLOW: &str = "interflow";

pub struct Interflow {
    id: FlowId,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    instruction_info: Option<InstructionInfo>,
}

pub fn get_interflow_id(n: &NodeData) -> Result<FlowId, serde_json::Error> {
    let id = n
        .targets_form
        .form_data
        .get("id")
        .unwrap_or(&JsonValue::Null);
    FlowId::deserialize(id)
}

impl Interflow {
    fn new(n: &NodeData) -> Result<Self, CommandError> {
        let id = get_interflow_id(n)?;
        let inputs = n
            .targets
            .iter()
            .map(|x| Input {
                name: x.name.clone(),
                type_bounds: [ValueType::Free].to_vec(),
                required: x.required,
                passthrough: x.passthrough,
            })
            .collect();

        let outputs = n
            .sources
            .iter()
            .map(|x| Output {
                name: x.name.clone(),
                r#type: ValueType::Free,
                optional: x.optional,
            })
            .collect();

        Ok(Self {
            id,
            inputs,
            outputs,
            instruction_info: n.instruction_info.clone(),
        })
    }
}

#[async_trait(?Send)]
impl CommandTrait for Interflow {
    fn name(&self) -> Name {
        INTERFLOW.into()
    }

    fn inputs(&self) -> Vec<Input> {
        self.inputs.clone()
    }

    fn outputs(&self) -> Vec<Output> {
        self.outputs.clone()
    }

    async fn run(&self, ctx: CommandContext, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let start = ctx
            .get::<start_flow::Svc>()
            .ok_or_else(|| anyhow::anyhow!("start_flow::Svc not found"))?
            .clone();

        let parent_flow_execute = if self.instruction_info.is_some() {
            Some(ctx.raw().services.execute.clone())
        } else {
            None
        };

        let (_, handle) = start
            .ready_oneshot()
            .await?
            .call(start_flow::Request {
                flow_id: self.id,
                inputs,
                options: StartFlowOptions {
                    origin: ctx.new_interflow_origin(),
                    solana_client: Some(ctx.solana_config().clone()),
                    parent_flow_execute,
                    ..Default::default()
                },
            })
            .await?;
        let result = handle.await?;
        if result.flow_errors.is_empty() {
            Ok(result.output)
        } else {
            let mut errors = String::new();
            for error in result.flow_errors {
                errors += &error;
                errors += ";\n";
            }
            Err(anyhow!(errors))
        }
    }

    fn instruction_info(&self) -> Option<InstructionInfo> {
        self.instruction_info.clone()
    }
}

flow_lib::submit!(CommandDescription::new(INTERFLOW, |data: &NodeData| {
    Ok(Box::new(Interflow::new(data)?))
}));
