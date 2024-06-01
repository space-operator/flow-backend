use crate::{command::prelude::*, flow_registry::FlowRegistry};
use anyhow::anyhow;
use flow_lib::command::InstructionInfo;

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

#[async_trait]
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

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let registry = ctx
            .get::<FlowRegistry>()
            .ok_or_else(|| anyhow::anyhow!("FlowRegistry not found"))?;

        let (_, handle) = registry
            .start(
                self.id,
                inputs,
                None,
                false,
                ctx.new_interflow_origin()
                    .ok_or_else(|| anyhow::anyhow!("this is a bug"))?,
                Some(ctx.cfg.solana_client.clone()),
                Some(
                    ctx.command
                        .as_ref()
                        .ok_or_else(|| CommandError::msg("command context not found"))?
                        .svc
                        .clone(),
                ),
            )
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
