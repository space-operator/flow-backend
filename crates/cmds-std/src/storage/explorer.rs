use flow_lib::command::prelude::*;

pub const FIRE_EXPLORER: &str = "fileexplorer";

#[derive(Debug)]
pub struct ExplorerCommand {
    outputs: Vec<Output>,
    result: ValueSet,
}

impl ExplorerCommand {
    fn new(data: &NodeData) -> Self {
        let outputs = data
            .sources
            .iter()
            .map(|o| Output {
                name: o.name.clone(),
                r#type: o.r#type.clone(),
                optional: false,
            })
            .collect();
        Self {
            outputs,
            result: data
                .targets_form
                .form_data
                .as_object()
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| (k.clone(), Value::from(v.clone())))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl CommandTrait for ExplorerCommand {
    fn name(&self) -> Name {
        FIRE_EXPLORER.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [].to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        self.outputs.clone()
    }

    async fn run(&self, _: CommandContextX, _: ValueSet) -> Result<ValueSet, CommandError> {
        Ok(self.result.clone())
    }
}

flow_lib::submit!(CommandDescription::new(FIRE_EXPLORER, |data: &NodeData| {
    Ok(Box::new(ExplorerCommand::new(data)))
}));
