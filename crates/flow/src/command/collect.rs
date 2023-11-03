use crate::command::prelude::*;

#[derive(Debug, Clone)]
pub struct Collect;

pub const COLLECT: &str = "collect";

pub const ELEMENT: &str = "element";

pub const ARRAY: &str = "array";

#[async_trait]
impl CommandTrait for Collect {
    fn name(&self) -> Name {
        COLLECT.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [Input {
            name: ELEMENT.into(),
            type_bounds: [ValueType::Free].to_vec(),
            required: false,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [Output {
            name: ARRAY.into(),
            r#type: ValueType::Free,
        }]
        .to_vec()
    }

    async fn run(&self, _ctx: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let v = inputs
            .remove(ELEMENT)
            .unwrap_or_else(|| Value::Array(Vec::new()));
        if matches!(&v, Value::Array(_)) {
            Ok(value::map! {
                ARRAY => v,
            })
        } else {
            // FlowGraph must prepare input for this node correctly
            unreachable!();
            // Err(value::Error::invalid_type(v.unexpected(), &"array").into())
        }
    }
}

inventory::submit!(CommandDescription::new(COLLECT, |_| {
    Ok(Box::new(Collect))
}));
