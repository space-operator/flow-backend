use crate::command_capnp::{command_factory, command_trait, command_trait::run_params};
use capnp::capability::Promise;
use flow_lib::{
    command::{CommandDescription, CommandError, CommandTrait},
    config::client::NodeData,
    context::{CommandContext, CommandContextData, FlowServices, FlowSetServices},
    utils::tower_client::unimplemented_svc,
    value::{self, bincode_impl::map_from_bincode},
};
use futures::TryFutureExt;
use std::{borrow::Cow, collections::BTreeMap, str::Utf8Error, sync::Arc};
use thiserror::Error as ThisError;
use tokio::sync::Mutex;

#[derive(ThisError, Debug)]
enum Error {
    #[error(transparent)]
    RmpDecode(#[from] rmp_serde::decode::Error),
    #[error(transparent)]
    RmpEncode(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("data contain invalid UTF-8")]
    Utf8(#[from] Utf8Error),
    #[error("command is not available: {:?}", .0)]
    NotAvailable(String),
    #[error(transparent)]
    Cap(#[from] capnp::Error),
    #[error(transparent)]
    NewCommand(CommandError),
    #[error(transparent)]
    Run(CommandError),
}

impl From<Error> for capnp::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::Cap(error) => error,
            error => capnp::Error::failed(error.to_string()),
        }
    }
}

struct CommandFactoryImpl {
    availables: BTreeMap<Cow<'static, str>, CommandDescription>,
}

impl CommandFactoryImpl {
    fn init_impl(
        &mut self,
        params: command_factory::InitParams,
        mut results: command_factory::InitResults,
    ) -> Result<(), Error> {
        let name = params.get()?.get_name()?.to_str()?;
        if let Some(description) = self.availables.get(name) {
            let nd = params.get()?.get_nd()?;
            let nd: NodeData = rmp_serde::from_slice(nd)?;
            let cmd = (description.fn_new)(&nd).map_err(Error::NewCommand)?;
            let cmd = Arc::new(Mutex::new(cmd));
            results
                .get()
                .set_cmd(capnp_rpc::new_client(CommandTraitImpl { cmd }));
            Ok(())
        } else {
            Err(Error::NotAvailable(name.to_owned()))
        }
    }
}

impl command_factory::Server for CommandFactoryImpl {
    fn init(
        &mut self,
        params: command_factory::InitParams,
        results: command_factory::InitResults,
    ) -> Promise<(), ::capnp::Error> {
        Promise::from_future(std::future::ready(
            self.init_impl(params, results).map_err(Into::into),
        ))
    }
}

struct CommandTraitImpl {
    cmd: Arc<Mutex<Box<dyn CommandTrait>>>,
}

fn parse_inputs(params: run_params::Reader<'_>) -> Result<value::Map, Error> {
    let inputs = params.get_inputs()?;
    Ok(map_from_bincode(inputs)?)
}

impl CommandTraitImpl {
    fn run_impl(
        &mut self,
        params: command_trait::RunParams,
        mut results: command_trait::RunResults,
    ) -> impl Future<Output = Result<(), Error>> + 'static {
        let cmd = self.cmd.clone();
        async move {
            let inputs = parse_inputs(params.get()?)?;
            let context = params.get()?.get_ctx()?;
            let data: CommandContextData = rmp_serde::from_slice(
                context
                    .data_request()
                    .send()
                    .promise
                    .await?
                    .get()?
                    .get_data()?,
            )?;
            let result = cmd
                .lock_owned()
                .await
                .run(
                    CommandContext::builder()
                        .execute(unimplemented_svc())
                        .get_jwt(unimplemented_svc())
                        .flow(FlowServices {
                            signer: unimplemented_svc(),
                            set: FlowSetServices {
                                http: reqwest::Client::new(),
                                solana_client: Arc::new(data.flow.set.solana.build_client()),
                                extensions: Arc::new(Default::default()),
                                api_input: unimplemented_svc(),
                            },
                        })
                        .data(data)
                        .build(),
                    inputs,
                )
                .await
                .map_err(Error::Run)?;
            results.get().set_output(&rmp_serde::to_vec_named(&result)?);
            Ok(())
        }
    }
}

impl command_trait::Server for CommandTraitImpl {
    fn run(
        &mut self,
        params: command_trait::RunParams,
        results: command_trait::RunResults,
    ) -> Promise<(), capnp::Error> {
        Promise::from_future(self.run_impl(params, results).map_err(Into::into))
    }
}

#[cfg(test)]
mod tests {
    use flow_lib::{
        CmdInputDescription, CmdOutputDescription, Name, ValueType,
        config::client::{Extra, TargetsForm},
        value::{Decimal, bincode_impl::map_to_bincode, with::AsDecimal},
    };
    use futures::{FutureExt, future::BoxFuture};
    use rust_decimal_macros::dec;
    use serde::Deserialize;

    use crate::command_capnp;

    use super::*;

    struct Add;
    impl CommandTrait for Add {
        fn name(&self) -> Name {
            "add".into()
        }

        fn inputs(&self) -> Vec<CmdInputDescription> {
            [
                CmdInputDescription {
                    name: "a".into(),
                    type_bounds: [ValueType::Decimal].into(),
                    required: true,
                    passthrough: false,
                },
                CmdInputDescription {
                    name: "b".into(),
                    type_bounds: [ValueType::Decimal].into(),
                    required: true,
                    passthrough: false,
                },
            ]
            .into()
        }

        fn outputs(&self) -> Vec<CmdOutputDescription> {
            [CmdOutputDescription {
                name: "c".into(),
                r#type: ValueType::Decimal,
                optional: false,
            }]
            .into()
        }
        fn run<'life0, 'async_trait>(
            &'life0 self,
            _: CommandContext,
            params: value::Map,
        ) -> BoxFuture<'async_trait, Result<value::Map, CommandError>>
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            async move {
                #[serde_with::serde_as]
                #[derive(Deserialize)]
                struct Input {
                    #[serde_as(as = "AsDecimal")]
                    a: Decimal,
                    #[serde_as(as = "AsDecimal")]
                    b: Decimal,
                }

                let x: Input = value::from_map(params)?;
                Ok(value::map! {
                    "c" => x.a + x.b,
                })
            }
            .boxed()
        }
    }

    #[tokio::test]
    async fn test_call() {
        let factory = CommandFactoryImpl {
            availables: [(
                Cow::Borrowed("add"),
                CommandDescription {
                    name: Cow::Borrowed("add"),
                    fn_new: |_| Ok(Box::new(Add)),
                },
            )]
            .into(),
        };
        let client = capnp_rpc::new_client::<command_capnp::command_factory::Client, _>(factory);
        let mut req = client.init_request();
        req.get().set_name("add");
        req.get().set_nd(
            rmp_serde::to_vec_named(&NodeData {
                r#type: flow_lib::CommandType::Native,
                node_id: String::new(),
                sources: Vec::new(),
                targets: Vec::new(),
                targets_form: TargetsForm {
                    form_data: serde_json::Value::Null,
                    extra: Extra {
                        ..Default::default()
                    },
                    wasm_bytes: None,
                },
                instruction_info: None,
            })
            .unwrap()
            .as_slice(),
        );
        let result = req.send().promise.await.unwrap();
        let cmd = result.get().unwrap().get_cmd().unwrap();
        let mut req = cmd.run_request();
        req.get().set_inputs(
            map_to_bincode(&value::map! {
                "a" => dec!(1.2888),
                "v" => dec!(3.5541),
            })
            .unwrap()
            .as_slice(),
        );
        let result = req.send().promise.await.unwrap();
    }
}
