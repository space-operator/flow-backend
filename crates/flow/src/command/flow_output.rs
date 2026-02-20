use crate::command::prelude::*;

#[derive(Debug)]
pub struct FlowOutputCommand {
    pub rename: Name,
}

pub const FLOW_OUTPUT: &str = "flow_output";

impl FlowOutputCommand {
    fn new(data: &NodeData) -> Self {
        let form = &data.targets_form.form_data;

        let rename = data
            .targets
            .first()
            .map(|t| t.name.clone())
            .filter(|name| !name.is_empty())
            .or_else(|| {
                form.get("label")
                    .and_then(|v| flow_lib::command::parse_value_tagged(v.clone()).ok())
                    .and_then(|v| match v {
                        Value::String(s) => Some(s),
                        _ => None,
                    })
            })
            .or_else(|| data.sources.first().map(|s| s.name.clone()))
            .unwrap_or_default();

        Self { rename }
    }
}

#[async_trait(?Send)]
impl CommandTrait for FlowOutputCommand {
    fn name(&self) -> Name {
        FLOW_OUTPUT.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [Input {
            name: self.rename.clone(),
            type_bounds: [ValueType::Free].to_vec(),
            required: true,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [Output {
            name: self.rename.clone(),
            r#type: ValueType::Free,
            optional: false,
        }]
        .to_vec()
    }

    async fn run(&self, _: CommandContext, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        Ok(match inputs.into_values().next() {
            Some(value) => ValueSet::from([(self.rename.clone(), value)]),
            None => ValueSet::new(),
        })
    }
}

flow_lib::submit!(CommandDescription::new(FLOW_OUTPUT, |data: &NodeData| {
    Ok(Box::new(FlowOutputCommand::new(data)))
}));

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::config::client::{Extra, Source, Target, TargetsForm};
    use serde_json::json;
    use uuid::Uuid;

    fn node_with_data(form_data: JsonValue, target_name: &str, source_name: &str) -> NodeData {
        NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: FLOW_OUTPUT.into(),
            sources: vec![Source {
                id: Uuid::new_v4(),
                name: source_name.to_owned(),
                r#type: ValueType::Free,
                optional: false,
            }],
            targets: vec![Target {
                id: Uuid::new_v4(),
                name: target_name.to_owned(),
                type_bounds: vec![ValueType::Free],
                required: true,
                passthrough: false,
            }],
            targets_form: TargetsForm {
                form_data,
                extra: Extra::default(),
                wasm_bytes: None,
            },
            instruction_info: None,
        }
    }

    #[test]
    fn target_name_has_priority_for_rename() {
        let node = node_with_data(json!({ "label": { "S": "ignored" } }), "Result", "Old");
        let cmd = FlowOutputCommand::new(&node);
        assert_eq!(cmd.rename, "Result");
    }

    #[test]
    fn reads_ivalue_label_when_target_missing() {
        let node = node_with_data(json!({ "label": { "S": "Renamed" } }), "", "Old");
        let cmd = FlowOutputCommand::new(&node);
        assert_eq!(cmd.rename, "Renamed");
    }

    #[test]
    fn falls_back_to_source_name() {
        let node = node_with_data(json!({}), "", "Output A");
        let cmd = FlowOutputCommand::new(&node);
        assert_eq!(cmd.rename, "Output A");
    }
}
