use flow_lib::{
    command::{CommandError, CommandFactory, CommandTrait, MatchCommand},
    config::client::NodeData,
};

use super::address_book::AddressBook;

pub struct CommandFactoryWithRemotes {
    pub factory: CommandFactory,
    pub remotes: Option<AddressBook>,
}

impl CommandFactoryWithRemotes {
    pub async fn init(
        &mut self,
        nd: &NodeData,
    ) -> Result<Option<Box<dyn CommandTrait>>, CommandError> {
        if let Some(remotes) = self.remotes.as_mut() {
            match remotes.init(nd).await {
                Ok(cmd) => match cmd {
                    Some(cmd) => return Ok(Some(cmd)),
                    None => {}
                },
                Err(error) => {
                    tracing::debug!("remote rpc error: {}", error);
                }
            }
        }

        self.factory.init(nd).await
    }

    pub fn availables(&self) -> impl Iterator<Item = MatchCommand> {
        self.factory
            .availables()
            .chain(self.remotes.iter().flat_map(|book| book.availables()))
    }
}
