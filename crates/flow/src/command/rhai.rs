use flow_lib::command::prelude::*;
use std::sync::Arc;

use crate::flow_registry::{run_rhai, FlowRegistry};

struct Command {
    name: Name,
    inner: Arc<rhai_script::Command>,
}

#[async_trait]
impl CommandTrait for Command {
    fn name(&self) -> Name {
        self.name.clone()
    }

    fn inputs(&self) -> Vec<Input> {
        self.inner.inputs.clone()
    }

    fn outputs(&self) -> Vec<Output> {
        self.inner.outputs.clone()
    }

    async fn run(&self, ctx: Context, input: ValueSet) -> Result<ValueSet, CommandError> {
        let registry = ctx
            .get::<FlowRegistry>()
            .ok_or_else(|| anyhow::anyhow!("FlowRegistry not found"))?;
        // TODO: stop token
        registry
            .run_rhai(run_rhai::Request {
                command: self.inner.clone(),
                ctx: ctx.clone(),
                input,
            })
            .await
    }
}

pub fn build(nd: &NodeData) -> Result<Box<dyn CommandTrait>, CommandError> {
    let inputs: Vec<Input> = nd.targets.iter().cloned().map(Into::into).collect();
    let outputs: Vec<Output> = nd.sources.iter().cloned().map(Into::into).collect();
    let source_code_name = inputs
        .get(0)
        .ok_or_else(|| CommandError::msg("no source code input"))?
        .name
        .clone();
    let cmd = Arc::new(rhai_script::Command {
        source_code_name,
        inputs,
        outputs,
    });

    Ok(Box::new(Command {
        name: nd.node_id.clone(),
        inner: cmd,
    }))
}
