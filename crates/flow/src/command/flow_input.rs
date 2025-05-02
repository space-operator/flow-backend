use crate::command::prelude::*;

#[derive(Debug)]
pub struct FlowInputCommand {
    label: Name,
}

pub const FLOW_INPUT: &str = "flow_input";

impl FlowInputCommand {
    fn new(data: &NodeData) -> Self {
        let form = &data.targets_form.form_data;

        let label = form
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        Self { label }
    }
}

#[async_trait]
impl CommandTrait for FlowInputCommand {
    fn name(&self) -> Name {
        FLOW_INPUT.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [].to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [Output {
            name: self.label.clone(),
            r#type: ValueType::Free,
            optional: false,
        }]
        .to_vec()
    }

    async fn run(&self, _: CommandContext, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let value = inputs.swap_remove(&self.label).unwrap_or(Value::Null);
        Ok(value::map! {
            &self.label => value,
        })
    }

    fn read_form_data(&self, data: JsonValue) -> ValueSet {
        data.get("form_label")
            .map(|value| {
                // TODO: is this a good way to do it?
                value::map! {
                    &self.label => value.clone(),
                }
            })
            .unwrap_or_default()
    }
}

flow_lib::submit!(CommandDescription::new(FLOW_INPUT, |data: &NodeData| {
    Ok(Box::new(FlowInputCommand::new(data)))
}));
