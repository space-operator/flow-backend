use flow_lib::command::prelude::*;

#[derive(Debug)]
pub struct NoteCommand {}

const NOTE: &str = "note";

#[async_trait]
impl CommandTrait for NoteCommand {
    fn name(&self) -> Name {
        NOTE.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [].to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [].to_vec()
    }

    async fn run(&self, _ctx: Context, _inputs: ValueSet) -> Result<ValueSet, CommandError> {
        Ok(ValueSet::new())
    }
}

flow_lib::submit!(CommandDescription::new(NOTE, |_| Ok(Box::new(
    NoteCommand {}
))));
