use crate::command::prelude::*;

#[derive(Debug)]
pub struct FlowInputCommand {
    label: Name,
}

pub const FLOW_INPUT: &str = "flow_input";

impl FlowInputCommand {
    fn new(data: &NodeData) -> Self {
        let form = &data.config;

        let label = data
            .outputs
            .first()
            .map(|s| s.name.clone())
            .filter(|name| !name.is_empty())
            .or_else(|| {
                form.get("label")
                    .and_then(|v| flow_lib::command::parse_value_tagged(v.clone()).ok())
                    .and_then(|v| match v {
                        Value::String(s) => Some(s),
                        _ => None,
                    })
            })
            .unwrap_or_default();

        Self { label }
    }
}

#[async_trait(?Send)]
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

    fn read_config(&self, data: JsonValue) -> ValueSet {
        if let Some(value) = data.get("value").or_else(|| data.get("form_label")) {
            let parsed = flow_lib::command::parse_value_tagged_or_json(value.clone());
            return value::map! {
                &self.label => parsed,
            };
        }

        ValueSet::new()
    }
}

flow_lib::submit!(CommandDescription::new(FLOW_INPUT, |data: &NodeData| {
    Ok(Box::new(FlowInputCommand::new(data)))
}));

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::config::client::OutputPort;
    use serde_json::json;
    use uuid::Uuid;

    fn test_node(config: JsonValue, source_name: &str) -> NodeData {
        NodeData {
            r#type: flow_lib::CommandType::Native,
            node_id: FLOW_INPUT.into(),
            outputs: vec![OutputPort {
                id: Uuid::new_v4(),
                name: source_name.to_owned(),
                r#type: ValueType::Free,
                optional: false,
                tooltip: None,
            }],
            inputs: Vec::new(),
            config,
            wasm: None,
            instruction_info: None,
        }
    }

    #[test]
    fn source_name_has_priority_for_label() {
        let node = test_node(json!({ "label": { "S": "Ignored" } }), "Amount");
        let cmd = FlowInputCommand::new(&node);
        assert_eq!(cmd.label, "Amount");
    }

    #[test]
    fn reads_ivalue_label_when_source_missing() {
        let node = test_node(json!({ "label": { "S": "Input Amount" } }), "");
        let cmd = FlowInputCommand::new(&node);
        assert_eq!(cmd.label, "Input Amount");
    }

    #[test]
    fn read_config_parses_ivalue_value() {
        let node = test_node(json!({ "label": { "S": "amount" } }), "");
        let cmd = FlowInputCommand::new(&node);
        let values = cmd.read_config(json!({ "value": { "U": "1000" } }));
        assert_eq!(values.get("amount"), Some(&Value::U64(1000)));
    }

    #[test]
    fn read_config_accepts_plain_json_value() {
        let node = test_node(json!({ "label": { "S": "amount" } }), "");
        let cmd = FlowInputCommand::new(&node);
        let values = cmd.read_config(json!({ "value": 1000 }));
        assert_eq!(values.get("amount"), Some(&Value::U64(1000)));
    }

    #[test]
    fn read_config_accepts_form_label() {
        let node = test_node(json!({ "label": { "S": "password" } }), "");
        let cmd = FlowInputCommand::new(&node);
        let values = cmd.read_config(json!({ "form_label": "Hunter1!" }));
        assert_eq!(values.get("password"), Some(&Value::String("Hunter1!".to_owned())));
    }
}
