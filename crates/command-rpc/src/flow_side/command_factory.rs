use flow_lib::{
    command::{CommandError, CommandFactory, CommandTrait, MatchCommand},
    config::client::NodeData,
};

use super::address_book::AddressBook;

#[derive(Clone)]
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
                Ok(cmd) => {
                    if let Some(cmd) = cmd {
                        return Ok(Some(cmd));
                    }
                }
                Err(error) => {
                    tracing::error!("remote rpc error for node {}: {:#}", nd.node_id, error);
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
