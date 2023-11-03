use std::collections::BTreeMap;
use thiserror::Error as ThisError;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MathOperation;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    operator: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    result_f64: f64,
    result_u64: u64,
    result_i64: i64,
    result_string: String,
}

// Name
const MATH_OPERATION: &str = "math_operation";

// Inputs
const NUMBER_1: &str = "number_1";
const NUMBER_2: &str = "number_2";
const OPERATOR: &str = "operator";

// Outputs
const RESULT_F64: &str = "result_u64";
const RESULT_U64: &str = "result_f64";
const RESULT_I64: &str = "result_i64";
const RESULT_STRING: &str = "result_string";

#[derive(Debug, ThisError)]
enum MathError {
    #[error("second argument can not be deserialized an object of values")]
    Decode(serde_json::Error),
    #[error(transparent)]
    Compute(#[from] fasteval::Error),
}

#[async_trait]
impl CommandTrait for MathOperation {
    fn name(&self) -> Name {
        MATH_OPERATION.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: NUMBER_1.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: NUMBER_2.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: OPERATOR.into(),
                type_bounds: [ValueType::String].to_vec(),
                required: false,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [
            CmdOutput {
                name: RESULT_F64.into(),
                r#type: ValueType::F64,
            },
            CmdOutput {
                name: RESULT_U64.into(),
                r#type: ValueType::U64,
            },
            CmdOutput {
                name: RESULT_I64.into(),
                r#type: ValueType::I64,
            },
            CmdOutput {
                name: RESULT_STRING.into(),
                r#type: ValueType::String,
            },
        ]
        .to_vec()
    }

    async fn run(&self, _ctx: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input { operator } = value::from_map(inputs.clone())?;

        let number_1 = inputs.remove(NUMBER_1).unwrap_or(value::Value::U64(0));
        let number_1 = match number_1 {
            Value::Decimal(v) => v.to_string(),
            Value::U64(v) => v.to_string(),
            Value::I64(v) => v.to_string(),
            Value::U128(v) => v.to_string(),
            Value::I128(v) => v.to_string(),
            Value::F64(v) => v.to_string(),
            Value::String(s) => s,
            other => serde_json::to_string_pretty(&other).unwrap(),
        };

        let number_2 = inputs.remove(NUMBER_2).unwrap_or(value::Value::U64(0));
        let number_2 = match number_2 {
            Value::Decimal(v) => v.to_string(),
            Value::U64(v) => v.to_string(),
            Value::I64(v) => v.to_string(),
            Value::U128(v) => v.to_string(),
            Value::I128(v) => v.to_string(),
            Value::F64(v) => v.to_string(),
            Value::String(s) => s,
            other => serde_json::to_string_pretty(&other).unwrap(),
        };

        // Variable Map
        let mut map: BTreeMap<String, f64> = BTreeMap::new();
        map.insert(
            "x".to_string(),
            serde_json::from_str(&number_1).map_err(MathError::Decode)?,
        );
        map.insert(
            "y".to_string(),
            serde_json::from_str(&number_2).map_err(MathError::Decode)?,
        );

        let operator = match &*operator {
            "^" | "Exponentiation" => "^",
            "%" | "Modulo" => "%",
            "/" | "Division" => "/",
            "*" | "Multiplication" => "*",
            "-" | "Subtraction" => "-",
            "+" | "Addition" => "+",
            &_ => "+",
        };

        let expression = ["x", operator, "y"].join("");

        // Calculation
        let result_f64 = match fasteval::ez_eval(&expression, &mut map) {
            Ok(value) => value,
            Err(_error) => 0.0,
        };

        // Get other types if matching
        let result_u64 = match result_f64.fract() == 0.0 {
            true => result_f64 as u64,
            false => 0,
        };

        let result_i64 = match result_f64.is_sign_negative() && result_f64.fract() == 0.0 {
            true => result_f64 as i64,
            false => 0,
        };

        let result_string = result_f64.to_string();

        Ok(value::to_map(&Output {
            result_f64,
            result_u64,
            result_i64,
            result_string,
        })?)
    }
}

inventory::submit!(CommandDescription::new(MATH_OPERATION, |_| Ok(Box::new(
    MathOperation {}
))));

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_valid() {
//         let input = value::to_map(&Input {
//             number_1: 100,
//             number_2: 20,
//         })
//         .unwrap();
//         let output = Addition.run(Context::default(), input).await.unwrap();
//         let result = value::from_map::<Output>(output).unwrap().result;
//         dbg!(result);
//     }
// }
