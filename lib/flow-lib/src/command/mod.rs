//! [`CommandTrait`] and command [`builder`].
//!
//! To make a new [`native`][crate::config::CommandType::Native] command:
//! 1. Implement [`CommandTrait`], 2 ways;
//!     - Manually implement it to your types.
//!     - Use [`builder`] helper.
//! 2. Use [`inventory::submit`] with a [`CommandDescription`] to register the command at compile-time.

use crate::{
    CommandType, ValueType,
    config::{
        CmdInputDescription, CmdOutputDescription, Name, ValueSet, client::NodeData,
        node::Permissions,
    },
    context::CommandContext,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap, fmt::Display};
use value::Value;

pub mod builder;

/// Import common types for writing commands.
pub mod prelude {
    pub use crate::{
        CmdInputDescription, CmdInputDescription as Input, CmdOutputDescription,
        CmdOutputDescription as Output, FlowId, Name, ValueSet, ValueType,
        command::{
            CommandDescription, CommandError, CommandTrait, InstructionInfo,
            builder::{BuildResult, BuilderCache, BuilderError, CmdBuilder},
        },
        config::{client::NodeData, node::Permissions},
        context::CommandContext,
        solana::{Instructions, Keypair, Pubkey, Signature},
    };
    pub use async_trait::async_trait;
    pub use bytes::Bytes;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value as JsonValue;
    pub use serde_with::serde_as;
    pub use thiserror::Error as ThisError;
    pub use value::{
        self, Decimal, Value,
        with::{AsDecimal, AsKeypair, AsPubkey, AsSignature},
    };
}

/// Error type of commmands.
pub type CommandError = anyhow::Error;

/// Generic trait for implementing commands.
#[async_trait::async_trait(?Send)]
pub trait CommandTrait: 'static {
    /// Unique name to identify the command.
    fn name(&self) -> Name;

    /// List of inputs that the command can receive.
    fn inputs(&self) -> Vec<CmdInputDescription>;

    /// List of outputs that the command will return.
    fn outputs(&self) -> Vec<CmdOutputDescription>;

    /// Run the command.
    async fn run(&self, ctx: CommandContext, params: ValueSet) -> Result<ValueSet, CommandError>;

    /// Specify if and how would this command output Solana instructions.
    fn instruction_info(&self) -> Option<InstructionInfo> {
        None
    }

    /// Specify requested permissions of this command.
    fn permissions(&self) -> Permissions {
        Permissions::default()
    }

    /// Async `Drop` method.
    async fn destroy(&mut self) {}

    /// Specify how [`form_data`][crate::config::NodeConfig::form_data] are read.
    fn read_form_data(&self, data: serde_json::Value) -> ValueSet {
        let mut result = ValueSet::new();
        for i in self.inputs() {
            if let Some(json) = data.get(&i.name) {
                let value = Value::from(json.clone());
                result.insert(i.name.clone(), value);
            }
        }
        result
    }

    /// Specify how to convert inputs into passthrough outputs.
    fn passthrough_outputs(&self, inputs: &ValueSet) -> ValueSet {
        let mut res = ValueSet::new();
        for i in self.inputs() {
            if i.passthrough {
                if let Some(value) = inputs.get(&i.name) {
                    if !i.required && matches!(value, Value::Null) {
                        continue;
                    }

                    let value = match i.type_bounds.first() {
                        Some(ValueType::Pubkey) => {
                            // keypair could be automatically converted into pubkey
                            // we don't want to passthrough the keypair here, only pubkey
                            value::pubkey::deserialize(value.clone()).map(Into::into)
                        }
                        _ => Ok(value.clone()),
                    }
                    .unwrap_or_else(|error| {
                        tracing::warn!("error reading passthrough: {}", error);
                        value.clone()
                    });
                    res.insert(i.name, value);
                }
            }
        }
        res
    }

    fn input_is_required(&self, name: &str) -> Option<bool> {
        self.inputs()
            .into_iter()
            .find_map(|i| (i.name == name).then_some(i.required))
    }

    fn output_is_optional(&self, name: &str) -> Option<bool> {
        self.outputs()
            .into_iter()
            .find_map(|o| (o.name == name).then_some(o.optional))
            .or_else(|| {
                self.inputs()
                    .into_iter()
                    .find_map(|i| (i.name == name && i.passthrough).then_some(!i.required))
            })
    }
}

/// Specify the order with which a command will return its output:
/// - [`before`][InstructionInfo::before]: list of output names returned before instructions are sent.
/// - [`signature`][InstructionInfo::signature]: name of the signature output port.
/// - [`after`][InstructionInfo::after]: list of output names returned after instructions are sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionInfo {
    pub before: Vec<Name>,
    pub signature: Name,
    pub after: Vec<Name>,
}

impl InstructionInfo {
    /// Simple `InstructionInfo` that can describe most commands:
    /// - [`before`][InstructionInfo::before]: All passthroughs and outputs, except for `signature`.
    /// - [`after`][InstructionInfo::after]: empty.
    pub fn simple<C: CommandTrait>(cmd: &C, signature: &str) -> Self {
        let before = cmd
            .inputs()
            .into_iter()
            .filter(|i| i.passthrough)
            .map(|i| i.name)
            .chain(
                cmd.outputs()
                    .into_iter()
                    .filter(|o| o.name != signature)
                    .map(|o| o.name),
            )
            .collect();
        Self {
            before,
            after: Vec::new(),
            signature: signature.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, bincode::Encode)]
pub enum MatchName {
    Exact(Cow<'static, str>),
    Regex(Cow<'static, str>),
}

// generated by bincode macro
impl<__Context> ::bincode::Decode<__Context> for MatchName {
    fn decode<__D: ::bincode::de::Decoder<Context = __Context>>(
        decoder: &mut __D,
    ) -> core::result::Result<Self, ::bincode::error::DecodeError> {
        let variant_index = <u32 as ::bincode::Decode<__D::Context>>::decode(decoder)?;
        match variant_index {
            0u32 => core::result::Result::Ok(Self::Exact {
                0: ::bincode::Decode::<__D::Context>::decode(decoder)?,
            }),
            1u32 => core::result::Result::Ok(Self::Regex {
                0: ::bincode::Decode::<__D::Context>::decode(decoder)?,
            }),
            variant => {
                core::result::Result::Err(::bincode::error::DecodeError::UnexpectedVariant {
                    found: variant,
                    type_name: "MatchName",
                    allowed: &::bincode::error::AllowedEnumVariants::Range { min: 0, max: 1 },
                })
            }
        }
    }
}

impl Display for MatchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchName::Exact(cow) => cow.fmt(f),
            MatchName::Regex(cow) => cow.fmt(f),
        }
    }
}

/// Use [`inventory::submit`] to register commands at compile-time.
#[derive(Clone)]
pub struct CommandDescription {
    pub r#type: CommandType,
    /// Name of the command, must be equal to the value returned by [`CommandTrait::name`].
    pub name: MatchName,
    /// Function to initialize the command from a [`NodeData`].
    pub fn_new: fn(&NodeData) -> Result<Box<dyn CommandTrait>, CommandError>,
}

impl CommandDescription {
    pub const fn new(
        name: &'static str,
        fn_new: fn(&NodeData) -> Result<Box<dyn CommandTrait>, CommandError>,
    ) -> Self {
        Self {
            r#type: CommandType::Native,
            name: MatchName::Exact(Cow::Borrowed(name)),
            fn_new,
        }
    }
}

inventory::collect!(CommandDescription);

pub fn collect_commands() -> BTreeMap<MatchName, &'static CommandDescription> {
    inventory::iter::<CommandDescription>()
        .map(|c| (c.name.clone(), c))
        .collect()
}
