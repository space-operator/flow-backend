use flow_lib::command::{MatchCommand, prelude::*};
use std::sync::Arc;
use tower::{Service, ServiceExt};

use crate::flow_registry::run_rhai;

struct Command {
    name: Name,
    inner: Arc<rhai_script::Command>,
}

#[async_trait(?Send)]
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

    async fn run(&self, ctx: CommandContext, input: ValueSet) -> Result<ValueSet, CommandError> {
        ctx.get::<run_rhai::Svc>()
            .ok_or_else(|| anyhow::anyhow!("run_rhai::Svc not found"))?
            .clone()
            .ready_oneshot()
            .await?
            .call(run_rhai::Request {
                command: self.inner.clone(),
                ctx: ctx.clone(),
                input,
            })
            .await
    }
}

pub fn build(nd: &NodeData) -> Result<Box<dyn CommandTrait>, CommandError> {
    let inputs: Vec<Input> = nd.targets.iter().cloned().map(Into::into).collect();
    let outputs: Vec<Output> = nd
        .sources
        .iter()
        .cloned()
        .map(|s| Output {
            // TODO: we did not upload this field to db
            optional: true,
            ..Output::from(s)
        })
        .collect();
    let source_code_name = inputs
        .first()
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

inventory::submit!(CommandDescription {
    matcher: MatchCommand {
        r#type: flow_lib::CommandType::Native,
        name: flow_lib::command::MatchName::Regex(std::borrow::Cow::Borrowed(
            "^(?:@spo/)?rhai_script(?:_|$)",
        ))
    },
    fn_new: futures::future::Either::Left(build)
});
