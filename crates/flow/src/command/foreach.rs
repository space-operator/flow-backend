use crate::command::prelude::*;

#[derive(Debug, Clone)]
pub struct Foreach;

pub const FOREACH: &str = "foreach";

pub const ARRAY: &str = "array";

pub const ELEMENT: &str = "element";

#[async_trait]
impl CommandTrait for Foreach {
    fn name(&self) -> Name {
        FOREACH.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [Input {
            name: ARRAY.into(),
            type_bounds: [ValueType::Free].to_vec(),
            required: true,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [Output {
            name: ELEMENT.into(),
            r#type: ValueType::Free,
            optional: false,
        }]
        .to_vec()
    }

    async fn run(&self, _ctx: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let v = inputs
            .remove(ARRAY)
            .ok_or_else(|| crate::Error::ValueNotFound(ARRAY.into()))?;
        if matches!(&v, Value::Array(_)) {
            Ok(value::map! {
                ELEMENT => v,
            })
        } else {
            // if it's not an array, treat it as a 1-element array.
            Ok(value::map! {
                ELEMENT => Value::Array([v].to_vec()),
            })
        }
    }
}

flow_lib::submit!(CommandDescription::new(FOREACH, |_| {
    Ok(Box::new(Foreach))
}));
