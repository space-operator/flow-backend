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
//!   "version": "0.1",
//!   "name": "add",
//!   "type": "native",
//!   "author_handle": "spo",
//!   "ports": {
//!     "inputs": [
//!       {
//!         "name": "a",
//!         "type_bounds": ["i64"],
//!         "required": true,
//!         "passthrough": false
//!       },
//!       {
//!         "name": "b",
//!         "type_bounds": ["i64"],
//!         "required": true,
//!         "passthrough": false
//!       }
//!     ],
//!     "outputs": [
//!       {
//!         "name": "result",
//!         "type": "i64"
//!       }
//!     ]
//!   }
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

/// Use this to cache computation such as parsing a node-definition.
pub type BuilderCache = LazyLock<Result<CmdBuilder, BuilderError>>;

/// Create a command from node-definition file and an `async fn run()` function.
#[derive(Debug, Clone)]
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
    /// Start building command with a node-definition (JSON or JSONC).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ValueType, node::Permissions};

    async fn noop(
        _: CommandContext,
        _: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        Ok(serde_json::Value::Null)
    }

    /// Shared V2 JSONC with rich port metadata for multiple tests.
    const V2_RICH: &str = r#"{
        // V2 definition with varied port attributes
        "version": "0.1",
        "name": "test_ports",
        "type": "native",
        "author_handle": "tester",
        "ports": {
            "inputs": [
                {
                    "name": "fee_payer",
                    "type_bounds": ["keypair"],
                    "required": true,
                    "passthrough": true
                },
                {
                    "name": "amount",
                    "type_bounds": ["u64", "decimal"],
                    "required": true,
                    "passthrough": false
                },
                {
                    "name": "memo",
                    "type_bounds": ["string"],
                    "required": false,
                    "passthrough": false
                }
            ],
            "outputs": [
                {
                    "name": "signature",
                    "type": "signature"
                },
                {
                    "name": "receipt",
                    "type": "free",
                    "optional": true
                }
            ]
        },
        "config_schema": {},
        "config": {}
    }"#;

    // ── Gap 1: Port round-trip ──────────────────────────────────────────

    #[test]
    fn v2_port_round_trip() {
        let cmd = CmdBuilder::new(V2_RICH)
            .unwrap()
            .check_name("@tester/test_ports")
            .unwrap()
            .build(noop);

        assert_eq!(cmd.name(), "@tester/test_ports");

        // Inputs
        let inputs = cmd.inputs();
        assert_eq!(inputs.len(), 3);

        assert_eq!(inputs[0].name, "fee_payer");
        assert_eq!(inputs[0].type_bounds, vec![ValueType::Keypair]);
        assert!(inputs[0].required);
        assert!(inputs[0].passthrough);

        assert_eq!(inputs[1].name, "amount");
        assert_eq!(
            inputs[1].type_bounds,
            vec![ValueType::U64, ValueType::Decimal]
        );
        assert!(inputs[1].required);
        assert!(!inputs[1].passthrough);

        assert_eq!(inputs[2].name, "memo");
        assert_eq!(inputs[2].type_bounds, vec![ValueType::String]);
        assert!(!inputs[2].required);
        assert!(!inputs[2].passthrough);

        // Outputs
        let outputs = cmd.outputs();
        assert_eq!(outputs.len(), 2);

        assert_eq!(outputs[0].name, "signature");
        assert_eq!(outputs[0].r#type, ValueType::Signature);
        assert!(!outputs[0].optional);

        assert_eq!(outputs[1].name, "receipt");
        assert_eq!(outputs[1].r#type, ValueType::Free);
        assert!(outputs[1].optional);
    }

    // ── Gap 2: Permissions survive build ────────────────────────────────

    #[test]
    fn v2_permissions_default_false() {
        let cmd = CmdBuilder::new(V2_RICH)
            .unwrap()
            .check_name("@tester/test_ports")
            .unwrap()
            .build(noop);

        assert!(!cmd.permissions().user_tokens);
    }

    #[test]
    fn v2_permissions_survive_build() {
        let cmd = CmdBuilder::new(V2_RICH)
            .unwrap()
            .check_name("@tester/test_ports")
            .unwrap()
            .permissions(Permissions { user_tokens: true })
            .build(noop);

        assert!(cmd.permissions().user_tokens);
    }

    // ── Gap 3: simple_instruction_info with V2 ─────────────────────────

    #[test]
    fn v2_simple_instruction_info() {
        let cmd = CmdBuilder::new(V2_RICH)
            .unwrap()
            .check_name("@tester/test_ports")
            .unwrap()
            .simple_instruction_info("signature")
            .unwrap()
            .build(noop);

        let info = cmd
            .instruction_info()
            .expect("instruction_info should be Some");
        assert_eq!(info.signature, "signature");
        // before = passthroughs (fee_payer) + non-signature outputs (receipt)
        assert_eq!(info.before, vec!["fee_payer", "receipt"]);
        assert!(info.after.is_empty());
    }

    // ── Gap 4: V2 defaults for omitted fields ──────────────────────────

    #[test]
    fn v2_defaults_for_omitted_fields() {
        let minimal = r#"{
            "version": "0.1",
            "name": "defaults_test",
            "type": "native",
            "author_handle": "spo",
            "ports": {
                "inputs": [{ "name": "x" }],
                "outputs": [{ "name": "y" }]
            }
        }"#;

        let cmd = CmdBuilder::new(minimal)
            .unwrap()
            .check_name("defaults_test")
            .unwrap()
            .build(noop);

        let inp = &cmd.inputs()[0];
        assert_eq!(inp.name, "x");
        assert!(inp.type_bounds.is_empty());
        assert!(!inp.required);
        assert!(!inp.passthrough);

        let out = &cmd.outputs()[0];
        assert_eq!(out.name, "y");
        assert_eq!(out.r#type, ValueType::Free);
        assert!(!out.optional);
    }

    // ── Gap 5: Error paths ─────────────────────────────────────────────

    #[test]
    fn check_name_wrong_name_errors() {
        let err = CmdBuilder::new(V2_RICH)
            .unwrap()
            .check_name("wrong")
            .unwrap_err();

        assert!(matches!(err, BuilderError::WrongName(_)));
    }

    #[test]
    fn check_name_strips_spo_prefix() {
        let spo_def = r#"{
            "version": "0.1",
            "name": "my_cmd",
            "type": "native",
            "author_handle": "spo",
            "ports": { "inputs": [], "outputs": [] }
        }"#;

        // Plain name matches @spo/my_cmd via prefix stripping
        CmdBuilder::new(spo_def)
            .unwrap()
            .check_name("my_cmd")
            .unwrap();

        // Fully qualified also matches
        CmdBuilder::new(spo_def)
            .unwrap()
            .check_name("@spo/my_cmd")
            .unwrap();
    }

    #[test]
    fn check_name_no_strip_for_non_spo() {
        let alice_def = r#"{
            "version": "0.1",
            "name": "my_cmd",
            "type": "native",
            "author_handle": "alice",
            "ports": { "inputs": [], "outputs": [] }
        }"#;

        // Plain name should NOT match @alice/my_cmd
        let err = CmdBuilder::new(alice_def)
            .unwrap()
            .check_name("my_cmd")
            .unwrap_err();
        assert!(matches!(err, BuilderError::WrongName(_)));
    }

    #[test]
    fn simple_instruction_info_output_not_found() {
        let err = CmdBuilder::new(V2_RICH)
            .unwrap()
            .simple_instruction_info("nonexistent")
            .unwrap_err();

        assert!(matches!(err, BuilderError::OutputNotFound(_)));
    }

    #[test]
    fn v2_malformed_missing_name() {
        let bad = r#"{ "type": "native", "author_handle": "spo", "ports": { "inputs": [], "outputs": [] } }"#;
        // Missing "name" — should fail to parse as either legacy or V2
        assert!(CmdBuilder::new(bad).is_err());
    }

    #[test]
    fn v2_malformed_invalid_json() {
        assert!(CmdBuilder::new("not json at all {{{").is_err());
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn v2_instruction_info_none_without_simple_instruction_info() {
        let cmd = CmdBuilder::new(V2_RICH)
            .unwrap()
            .check_name("@tester/test_ports")
            .unwrap()
            .build(noop);

        assert!(cmd.instruction_info().is_none());
    }

    #[test]
    fn v2_instruction_info_no_passthroughs() {
        // No passthrough inputs — before should only contain non-signature outputs
        let def = r#"{
            "version": "0.1",
            "name": "no_pt",
            "type": "native",
            "author_handle": "spo",
            "ports": {
                "inputs": [
                    { "name": "data", "type_bounds": ["string"], "required": true, "passthrough": false }
                ],
                "outputs": [
                    { "name": "signature", "type": "signature" },
                    { "name": "extra", "type": "pubkey" }
                ]
            }
        }"#;

        let cmd = CmdBuilder::new(def)
            .unwrap()
            .simple_instruction_info("signature")
            .unwrap()
            .build(noop);

        let info = cmd.instruction_info().unwrap();
        assert_eq!(info.signature, "signature");
        // No passthroughs, so before = only non-signature outputs
        assert_eq!(info.before, vec!["extra"]);
        assert!(info.after.is_empty());
    }

    #[test]
    fn v2_instruction_info_signature_only_output() {
        // Single output = signature, no passthroughs → before is empty
        let def = r#"{
            "version": "0.1",
            "name": "sig_only",
            "type": "native",
            "author_handle": "spo",
            "ports": {
                "inputs": [
                    { "name": "data", "required": true }
                ],
                "outputs": [
                    { "name": "signature", "type": "signature" }
                ]
            }
        }"#;

        let cmd = CmdBuilder::new(def)
            .unwrap()
            .simple_instruction_info("signature")
            .unwrap()
            .build(noop);

        let info = cmd.instruction_info().unwrap();
        assert_eq!(info.signature, "signature");
        assert!(info.before.is_empty());
        assert!(info.after.is_empty());
    }

    #[test]
    fn v2_builder_permissions_override_definition() {
        // Definition has user_tokens: true, builder overrides to false
        let def = r#"{
            "version": "0.1",
            "name": "override_test",
            "type": "native",
            "author_handle": "spo",
            "permissions": { "user_tokens": true },
            "ports": { "inputs": [], "outputs": [] }
        }"#;

        let cmd = CmdBuilder::new(def)
            .unwrap()
            .permissions(Permissions { user_tokens: false })
            .build(noop);

        assert!(!cmd.permissions().user_tokens);
    }

    #[test]
    fn v2_unknown_value_type_parses_as_other() {
        let def = r#"{
            "version": "0.1",
            "name": "other_type",
            "type": "native",
            "author_handle": "spo",
            "ports": {
                "inputs": [{ "name": "x", "type_bounds": ["custom_thing"] }],
                "outputs": [{ "name": "y", "type": "another_custom" }]
            }
        }"#;

        let cmd = CmdBuilder::new(def).unwrap().build(noop);

        assert_eq!(cmd.inputs()[0].type_bounds, vec![ValueType::Other]);
        assert_eq!(cmd.outputs()[0].r#type, ValueType::Other);
    }

    #[test]
    fn v2_builder_always_produces_native_type() {
        // CmdBuilder is only used for native Rust commands.
        // The definition's "type" field is parsed but the built command
        // always returns Native via the default CommandTrait::r#type().
        let def = r#"{
            "version": "0.1",
            "name": "deno_cmd",
            "type": "deno",
            "author_handle": "spo",
            "ports": { "inputs": [], "outputs": [] }
        }"#;

        let cmd = CmdBuilder::new(def).unwrap().build(noop);
        assert_eq!(cmd.r#type(), crate::config::CommandType::Native);
    }
}
