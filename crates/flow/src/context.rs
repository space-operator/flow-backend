use crate::{
    command::wasm::{Description, WasmCommand},
    Error,
};
use flow_lib::{
    command::{CommandDescription, CommandTrait},
    config::client::NodeData,
    CommandType,
};
use std::{borrow::Cow, collections::BTreeMap};
use tokio::process::Child;

pub struct CommandFactory {
    pub natives: BTreeMap<Cow<'static, str>, CommandDescription>,
}

impl Default for CommandFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandFactory {
    pub fn new() -> Self {
        let mut natives = BTreeMap::new();
        for d in inventory::iter::<CommandDescription>() {
            let name = d.name.clone();
            if natives.insert(name.clone(), d.clone()).is_some() {
                tracing::error!("duplicated command {:?}", name);
            }
        }

        Self { natives }
    }

    pub fn new_native_command(
        &self,
        name: &str,
        config: &NodeData,
    ) -> crate::Result<Box<dyn CommandTrait>> {
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
        &self,
        name: &str,
        config: &NodeData,
        spawned: &mut Vec<Child>,
    ) -> crate::Result<Box<dyn CommandTrait>> {
        match config.r#type {
            CommandType::Mock => Err(Error::custom("mock node")),
            CommandType::Native => self.new_native_command(name, config),
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
