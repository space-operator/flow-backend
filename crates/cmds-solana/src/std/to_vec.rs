use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ToVec;

// Name
const TO_VEC: &str = "to_vec";

// Inputs
const FIRST: &str = "first";
const SECOND: &str = "second";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub first: Value,
    pub second: Option<Value>,
}

// Outputs
const RESULT: &str = "result";

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: Vec<Value>,
}

#[async_trait]
impl CommandTrait for ToVec {
    fn name(&self) -> Name {
        TO_VEC.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: FIRST.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: SECOND.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: RESULT.into(),
            r#type: ValueType::Array,
        }]
        .to_vec()
    }

    async fn run(&self, _ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input { first, second } = value::from_map::<Input>(inputs)?;

        let result = if let Some(second) = second {
            vec![first, second]
        } else {
            vec![first]
        };

        Ok(value::to_map(&Output { result })?)
    }
}

inventory::submit!(CommandDescription::new(TO_VEC, |_| Ok(Box::new(ToVec {}))));
