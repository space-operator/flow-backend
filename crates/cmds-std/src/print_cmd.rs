use anyhow::anyhow;
use flow_lib::command::prelude::*;

#[derive(Debug)]
pub struct PrintCommand {}

pub const PRINT_CMD: &str = "print";

// Inputs
pub const PRINT: &str = "print";

// Outputs
pub const PRINT_OUTPUT: &str = "__print_output";

#[async_trait]
impl CommandTrait for PrintCommand {
    fn name(&self) -> Name {
        PRINT_CMD.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [Input {
            name: PRINT.into(),
            type_bounds: [ValueType::Free].to_vec(),
            required: true,
            passthrough: true,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [Output {
            name: PRINT_OUTPUT.into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, _ctx: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input = inputs
            .remove(PRINT)
            .ok_or_else(|| anyhow!("input not found: {}", PRINT))?;
        let output = match input {
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
        Ok(ValueSet::from([(
            PRINT_OUTPUT.into(),
            Value::String(output),
        )]))
    }
}

flow_lib::submit!(CommandDescription::new(PRINT_CMD, |_| Ok(Box::new(
    PrintCommand {}
))));
