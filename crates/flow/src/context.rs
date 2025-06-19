use crate::{
    Error,
    command::wasm::{Description, WasmCommand},
};
use command_rpc::flow_side::address_book::AddressBook;
use flow_lib::{
    CommandType,
    command::{CommandDescription, CommandTrait},
    config::client::NodeData,
};
use std::{borrow::Cow, collections::BTreeMap};
use tokio::process::Child;

pub struct CommandFactory {
    natives: BTreeMap<Cow<'static, str>, &'static CommandDescription>,
    rpc: Option<AddressBook>,
}

impl Default for CommandFactory {
    fn default() -> Self {
        Self::new(None)
    }
}

impl CommandFactory {
    pub fn new(rpc: Option<AddressBook>) -> Self {
        let mut natives = BTreeMap::new();
        for d in inventory::iter::<CommandDescription>() {
            let name = d.name.clone();
            if natives.insert(name.clone(), d).is_some() {
                tracing::error!("duplicated command {:?}", name);
            }
        }

        Self { natives, rpc }
    }

    pub fn avaiables(&self) -> impl Iterator<Item = &str> {
        self.natives.keys().map(|s| s.as_ref())
    }

    pub async fn new_native_command(
        &mut self,
        name: &str,
        config: &NodeData,
    ) -> crate::Result<Box<dyn CommandTrait>> {
        if let Some(rpc) = self.rpc.as_mut() {
            match rpc.new_command(name, config).await {
                Ok(cmd) => return Ok(cmd),
                Err(error) => tracing::debug!("rpc error: {}", error),
            }
        }
        match self.natives.get(name) {
            Some(d) => (d.fn_new)(config).map_err(crate::Error::CreateCmd),
            None => {
                if rhai_script::is_rhai_script(name) {
                    crate::command::rhai::build(config).map_err(crate::Error::CreateCmd)
                } else {
                    Err(Error::Any(format!("native not found: {}", name).into()))
                }
            }
        }
    }

    pub async fn new_deno_command(
        &self,
        config: &NodeData,
        spawned: &mut Vec<Child>,
    ) -> crate::Result<Box<dyn CommandTrait>> {
        let (cmd, child) = cmds_deno::new(config).await.map_err(Error::custom)?;
        spawned.push(child);
        Ok(cmd)
    }

    pub async fn new_command(
        &mut self,
        name: &str,
        config: &NodeData,
        spawned: &mut Vec<Child>,
    ) -> crate::Result<Box<dyn CommandTrait>> {
        match config.r#type {
            CommandType::Mock => Err(Error::custom("mock node")),
            CommandType::Native => self.new_native_command(name, config).await,
            CommandType::Deno => self.new_deno_command(config, spawned).await,
            CommandType::Wasm => {
                let bytes = config
                    .targets_form
                    .wasm_bytes
                    .clone()
                    .ok_or_else(|| Error::Any("wasm_bytes not found".into()))?;

                // Map inputs and outputs
                let inputs = config
                    .targets
                    .iter()
                    .map(|it| Description {
                        name: it.name.clone(),
                        r#type: it.type_bounds[0].clone(),
                    })
                    .collect();

                let outputs = config
                    .sources
                    .iter()
                    .map(|it| Description {
                        name: it.name.clone(),
                        r#type: it.r#type.clone(),
                    })
                    .collect();

                // Compile wasm and create command
                let command: Box<dyn CommandTrait> = Box::new(WasmCommand {
                    bytes,
                    function: String::from("main"),
                    inputs,
                    outputs,
                });
                Ok(command)
            }
        }
    }
}
