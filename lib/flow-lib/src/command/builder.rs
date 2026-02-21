//! Helper for building command from node-definition files.
//!
//! Node-definition files can be JSON or JSONC and may use either:
//! - legacy backend shape matching [`Definition`]
//! - V2 node-definition shape (converted internally into [`Definition`])
//!
//! # Example
//!
//! A command that adds 2 numbers:
//! ```
//! use flow_lib::command::{*, prelude::*, builder::*};
//!
//! inventory::submit!(CommandDescription::new("add", |_| build()));
//!
//! const DEFINITION: &str = r#"
//! {
//!   "type": "native",
//!   "data": {
//!     "node_id": "add"
//!   },
//!   "sources": [
//!     {
//!       "name": "result",
//!       "type": "i64"
//!     }
//!   ],
//!   "targets": [
//!     {
//!       "name": "a",
//!       "type_bounds": ["i64"],
//!       "required": true,
//!       "passthrough": false
//!     },
//!     {
//!       "name": "b",
//!       "type_bounds": ["i64"],
//!       "required": true,
//!       "passthrough": false
//!     }
//!   ]
//! }
//! "#;
//!
//! fn build() -> BuildResult {
//!     static CACHE: BuilderCache = BuilderCache::new(|| {
//!         CmdBuilder::new(DEFINITION)?
//!             .check_name("add")
//!     });
//!     Ok(CACHE.clone()?.build(run))
//! }
//!
//! #[derive(serde::Deserialize, Debug)]
//! struct Input {
//!     a: i64,
//!     b: i64,
//! }
//!
//! #[derive(serde::Serialize, Debug)]
//! struct Output {
//!     result: i64,
//! }
//!
//! async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
//!     Ok(Output { result: input.a + input.b })
//! }
//! ```

use super::{CommandError, CommandTrait, FnNewResult};
use crate::{
    Name,
    command::InstructionInfo,
    config::node::{Definition, Permissions, parse_definition},
    context::CommandContext,
    utils::LocalBoxFuture,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{future::Future, sync::LazyLock};
use thiserror::Error as ThisError;

/// `fn build() -> BuildResult`.
pub type BuildResult = FnNewResult;

/// Use this to cache computation such as parsing JSON node-definition.
pub type BuilderCache = LazyLock<Result<CmdBuilder, BuilderError>>;

/// Create a command from node-definition file and an `async fn run()` function.
#[derive(Clone)]
pub struct CmdBuilder {
    def: Definition,
    signature_name: Option<String>,
}

#[derive(ThisError, Debug, Clone)]
pub enum BuilderError {
    #[error("{0}")]
    Json(String),
    #[error("wrong command name: {0}")]
    WrongName(String),
    #[error("output not found: {0}")]
    OutputNotFound(String),
}

impl From<serde_json::Error> for BuilderError {
    fn from(value: serde_json::Error) -> Self {
        BuilderError::Json(value.to_string())
    }
}

impl CmdBuilder {
    /// Start building command with a JSON node-definition.
    /// Most of the time you would use [`include_str`] to get the file content and pass to this.
    pub fn new(def: &str) -> Result<Self, serde_json::Error> {
        let def = parse_definition(def)?;
        Ok(Self {
            def,
            signature_name: None,
        })
    }

    /// Check that the command name in node-definition is equal to this name, to prevent accidentally
    /// using the wrong node-definition.
    pub fn check_name(self, name: &str) -> Result<Self, BuilderError> {
        fn strip_spo_scope(s: &str) -> Option<&str> {
            s.strip_prefix("@spo/")
        }

        let actual = self.def.data.node_id.as_str();
        let matches = actual == name
            || strip_spo_scope(actual).is_some_and(|unscoped| unscoped == name)
            || strip_spo_scope(name).is_some_and(|unscoped| unscoped == actual);

        if matches {
            Ok(self)
        } else {
            Err(BuilderError::WrongName(self.def.data.node_id))
        }
    }

    /// Set permissions of the command.
    pub fn permissions(mut self, p: Permissions) -> Self {
        self.def.permissions = p;
        self
    }

    /// Use an [`InstructionInfo::simple`] for this command.
    pub fn simple_instruction_info(mut self, signature_name: &str) -> Result<Self, BuilderError> {
        if self.def.sources.iter().any(|x| x.name == signature_name) {
            self.signature_name = Some(signature_name.to_owned());
            Ok(self)
        } else {
            Err(BuilderError::OutputNotFound(signature_name.to_owned()))
        }
    }

    /// Build the command, `f` will be used as this command's [`fn run()`][CommandTrait::run].
    ///
    /// - `f` must be an `async fn(Context, Input) -> Result<Output, CommandError>`.
    /// - `Input` must implement [`DeserializeOwned`].
    /// - `Output` must implement [`Serialize`].
    pub fn build<T, U, Fut, F>(self, f: F) -> Box<dyn CommandTrait>
    where
        F: Fn(CommandContext, T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<U, CommandError>> + Send + 'static,
        T: DeserializeOwned + 'static,
        U: Serialize,
    {
        struct Command<T, Fut> {
            name: Name,
            inputs: Vec<crate::CmdInputDescription>,
            outputs: Vec<crate::CmdOutputDescription>,
            instruction_info: Option<InstructionInfo>,
            permissions: Permissions,
            run: Box<dyn Fn(CommandContext, T) -> Fut + Send + Sync + 'static>,
        }

        impl<T, U, Fut> CommandTrait for Command<T, Fut>
        where
            Fut: Future<Output = Result<U, CommandError>> + Send + 'static,
            T: DeserializeOwned + 'static,
            U: Serialize,
        {
            fn name(&self) -> Name {
                self.name.clone()
            }

            fn instruction_info(&self) -> Option<InstructionInfo> {
                self.instruction_info.clone()
            }

            fn inputs(&self) -> Vec<crate::CmdInputDescription> {
                self.inputs.clone()
            }

            fn outputs(&self) -> Vec<crate::CmdOutputDescription> {
                self.outputs.clone()
            }

            fn run<'a: 'b, 'b>(
                &'a self,
                ctx: CommandContext,
                params: crate::ValueSet,
            ) -> LocalBoxFuture<'b, Result<crate::ValueSet, CommandError>> {
                match value::from_map(params) {
                    Ok(input) => {
                        let fut = (self.run)(ctx, input);
                        Box::pin(async move { Ok(value::to_map(&fut.await?)?) })
                    }
                    Err(error) => Box::pin(async move { Err(error.into()) }),
                }
            }

            fn permissions(&self) -> Permissions {
                self.permissions.clone()
            }
        }

        let mut cmd = Command {
            name: self.def.data.node_id.clone(),
            run: Box::new(f),
            inputs: self
                .def
                .targets
                .into_iter()
                .map(|x| crate::CmdInputDescription {
                    name: x.name,
                    type_bounds: x.type_bounds,
                    required: x.required,
                    passthrough: x.passthrough,
                })
                .collect(),
            outputs: self
                .def
                .sources
                .into_iter()
                .map(|x| crate::CmdOutputDescription {
                    name: x.name,
                    r#type: x.r#type,
                    optional: x.optional,
                })
                .collect(),
            instruction_info: self.def.data.instruction_info,
            permissions: self.def.permissions,
        };

        if let Some(name) = self.signature_name {
            cmd.instruction_info = Some(InstructionInfo::simple(&cmd, &name))
        }

        Box::new(cmd)
    }
}
