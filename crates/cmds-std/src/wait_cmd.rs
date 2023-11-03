use flow_lib::command::prelude::*;

#[derive(Debug)]
pub struct WaitCommand {}

pub const WAIT_CMD: &str = "wait";

// Inputs
pub const VALUE: &str = "value";
pub const WAIT_FOR: &str = "wait_for";

#[async_trait]
impl CommandTrait for WaitCommand {
    fn name(&self) -> Name {
        WAIT_CMD.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [
            Input {
                name: VALUE.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: true,
            },
            Input {
                name: WAIT_FOR.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: true,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [].to_vec()
    }

    async fn run(&self, _ctx: Context, _inputs: ValueSet) -> Result<ValueSet, CommandError> {
        Ok(ValueSet::new())
    }
}

flow_lib::submit!(CommandDescription::new(WAIT_CMD, |_| Ok(Box::new(
    WaitCommand {}
))));
