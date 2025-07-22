use std::cell::LazyCell;
use std::sync::LazyLock;

use crate::command_side::command_factory::{self, CommandFactoryExt};
use crate::flow_side::command_context::CommandContextImpl;
use crate::tracing::TrackFlowRun;
use cmds_std as _;
use flow_lib::command::{CommandDescription, CommandError, CommandFactory, CommandTrait};
use flow_lib::config::client::NodeData;
use flow_lib::context::CommandContext;
use flow_lib::value;
use flow_lib::value::bincode_impl::map_from_bincode;
use flow_lib::{
    CmdInputDescription, CmdOutputDescription, Name, ValueType,
    config::client::{Extra, Source, Target, TargetsForm},
    utils::LocalBoxFuture,
    value::{Decimal, bincode_impl::map_to_bincode, with::AsDecimal},
};
use futures::FutureExt;
use iroh::{Endpoint, Watcher};
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

thread_local! {
    static TRACKER: TrackFlowRun = TrackFlowRun::init_tracing();
}

#[actix::test]
async fn test_serve_iroh() {
    let tracker = TRACKER.with(Clone::clone);
    let addr = {
        let factory = command_factory::new_client(CommandFactory::collect(), tracker);
        let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
        let addr = endpoint.node_addr().initialized().await.unwrap();
        factory.bind_iroh(endpoint);
        addr
    };

    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let client = command_factory::connect_iroh(endpoint, addr).await.unwrap();
    let names = client.all_availables().await.unwrap();
    dbg!(&names);
    assert!(!names.is_empty());
}

inventory::submit!(CommandDescription::new("add", |_| Ok(Box::new(Add))));

#[actix::test]
async fn test_call() {
    let tracker = TRACKER.with(Clone::clone);
    let client = command_factory::new_client(CommandFactory::collect(), tracker);
    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    dbg!("bind");
    let addr = endpoint.node_addr().initialized().await.unwrap();
    dbg!(&addr);
    client.bind_iroh(endpoint);

    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let client = command_factory::connect_iroh(endpoint, addr).await.unwrap();

    dbg!("connected");

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

    let cmd = client.init(&nd).await.unwrap().unwrap();

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
    assert_eq!(output["c"], dec!(4.8429).into());
    dbg!(output);
}
