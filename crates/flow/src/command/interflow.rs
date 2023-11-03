use crate::{command::prelude::*, flow_registry::FlowRegistry};

pub const INTERFLOW: &str = "interflow";

pub struct Interflow {
    id: FlowId,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
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
                required: false,
                passthrough: false,
            })
            .collect();

        let outputs = n
            .sources
            .iter()
            .map(|x| Output {
                name: x.name.clone(),
                r#type: ValueType::Free,
            })
            .collect();

        Ok(Self {
            id,
            inputs,
            outputs,
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
            )
            .await?;
        let result = handle.await?;
        Ok(result.output)
    }
}

inventory::submit!(CommandDescription::new(INTERFLOW, |data: &NodeData| {
    Ok(Box::new(Interflow::new(data)?))
}));
