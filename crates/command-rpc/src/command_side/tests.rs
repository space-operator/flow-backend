use super::*;
use crate::{command_capnp, flow_side::CommandContextImpl};
use cmds_std as _;
use flow_lib::{
    CmdInputDescription, CmdOutputDescription, Name, ValueType,
    command::collect_commands,
    config::client::{Extra, Source, Target, TargetsForm},
    utils::LocalBoxFuture,
    value::{Decimal, bincode_impl::map_to_bincode, with::AsDecimal},
};
use futures::FutureExt;
use iroh::SecretKey;
use rand::rngs::OsRng;
use rust_decimal_macros::dec;
use serde::Deserialize;

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
    ) -> LocalBoxFuture<'async_trait, Result<value::Map, CommandError>>
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

#[actix::test]
async fn test_serve() {
    let factory = CommandFactoryImpl {
        availables: collect_commands(),
        iroh_endpoint: None,
    };
    let (tx, rx) = oneshot::channel();
    spawn_local(serve(
        "127.0.0.1:0",
        capnp_rpc::new_client(factory),
        Some(tx),
    ));
    let addr = rx.await.unwrap();
    dbg!(addr);

    let client = connect_command_factory(addr).await.unwrap();
    let resp = client
        .all_availables_request()
        .send()
        .promise
        .await
        .unwrap();
    let data = resp.get().unwrap().get_availables().unwrap();
    let names: Vec<&str> = bincode::borrow_decode_from_slice(&data, standard())
        .unwrap()
        .0;
    dbg!(&names);
    assert!(!names.is_empty());
}

#[actix::test]
async fn test_serve_iroh() {
    let factory = CommandFactoryImpl {
        availables: collect_commands(),
        iroh_endpoint: None,
    };
    let factory_key = SecretKey::generate(rand::rngs::OsRng);
    let client = factory.serve_iroh(factory_key).await.unwrap().0;
    let addr = client.iroh_address().await.unwrap();

    let client = connect_iroh_command_factory(addr, SecretKey::generate(OsRng))
        .await
        .unwrap();
    let resp = client
        .all_availables_request()
        .send()
        .promise
        .await
        .unwrap();
    let data = resp.get().unwrap().get_availables().unwrap();
    let names: Vec<&str> = bincode::borrow_decode_from_slice(&data, standard())
        .unwrap()
        .0;
    dbg!(&names);
    assert!(!names.is_empty());
}

#[actix::test]
async fn test_call() {
    let factory = CommandFactoryImpl {
        availables: [(
            Cow::Borrowed("add"),
            &CommandDescription {
                name: Cow::Borrowed("add"),
                fn_new: |_| Ok(Box::new(Add)),
            },
        )]
        .into(),
        iroh_endpoint: None,
    };
    let client = capnp_rpc::new_client::<command_capnp::command_factory::Client, _>(factory);
    let mut req = client.init_request();
    req.get().set_name("add");
    let nd = NodeData {
        r#type: flow_lib::CommandType::Native,
        node_id: "add".to_owned(),
        sources: [Source {
            id: <_>::default(),
            name: "c".to_owned(),
            r#type: ValueType::Decimal,
            optional: false,
        }]
        .into(),
        targets: [
            Target {
                id: <_>::default(),
                name: "a".to_owned(),
                type_bounds: [ValueType::Decimal].into(),
                required: true,
                passthrough: false,
            },
            Target {
                id: <_>::default(),
                name: "b".to_owned(),
                type_bounds: [ValueType::Decimal].into(),
                required: true,
                passthrough: false,
            },
        ]
        .into(),
        targets_form: TargetsForm {
            form_data: serde_json::Value::Null,
            extra: Extra {
                ..Default::default()
            },
            wasm_bytes: None,
        },
        instruction_info: None,
    };
    req.get().set_nd(&simd_json::to_vec(&nd).unwrap());
    let result = req.send().promise.await.unwrap();
    let cmd = result.get().unwrap().get_cmd().unwrap();
    let mut req = cmd.run_request();
    req.get().set_inputs(
        map_to_bincode(&value::map! {
            "a" => dec!(1.2888),
            "b" => dec!(3.5541),
        })
        .unwrap()
        .as_slice(),
    );
    req.get().set_ctx(capnp_rpc::new_client(CommandContextImpl {
        context: CommandContext::test_context(),
    }));
    let result = req.send().promise.await.unwrap();
    let output = map_from_bincode(result.get().unwrap().get_output().unwrap()).unwrap();
    dbg!(output);
}
