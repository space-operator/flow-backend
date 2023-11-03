use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ToString;

const TO_STRING: &str = "to_string";

// Input
const STRINGIFY: &str = "stringify";

// Output
const RESULT: &str = "result";

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: String,
}

#[async_trait]
impl CommandTrait for ToString {
    fn name(&self) -> Name {
        TO_STRING.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [CmdInput {
            name: STRINGIFY.into(),
            type_bounds: [ValueType::Free].to_vec(),
            required: false,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: RESULT.into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, _: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input = inputs.remove(STRINGIFY).unwrap_or("".into());

        let result = match input {
            Value::Decimal(v) => v.to_string(),
            Value::U64(v) => v.to_string(),
            Value::I64(v) => v.to_string(),
            Value::U128(v) => v.to_string(),
            Value::I128(v) => v.to_string(),
            Value::F64(v) => v.to_string(),
            Value::B32(v) => bs58::encode(&v).into_string(),
            Value::B64(v) => bs58::encode(&v).into_string(),
            Value::String(s) => s,
            other => serde_json::to_string_pretty(&other).unwrap(),
        };

        // let result = serde_json::to_string(&output).unwrap();

        Ok(value::to_map(&Output { result })?)
    }
}

inventory::submit!(CommandDescription::new(TO_STRING, |_| Ok(Box::new(
    ToString
))));
