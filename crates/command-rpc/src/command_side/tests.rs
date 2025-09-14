use crate::command_side::command_factory::{self, CommandFactoryExt};
use crate::flow_side::command_context::CommandContextImpl;
use crate::tracing::TrackFlowRun;
use cmds_std as _;
use flow_lib::{
    CmdInputDescription, CmdOutputDescription, Name, ValueType,
    command::{CommandDescription, CommandError, CommandFactory, CommandTrait},
    context::CommandContext,
    utils::LocalBoxFuture,
    value::{
        self, Decimal, bincode_impl::map_from_bincode, bincode_impl::map_to_bincode,
        with::AsDecimal,
    },
};
use futures::FutureExt;
use iroh::{Endpoint, Watcher};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

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

            #[serde_with::serde_as]
            #[derive(Serialize)]
            struct Output {
                #[serde_as(as = "AsDecimal")]
                c: Decimal,
            }

            let Input { a, b } = value::from_map(params)?;
            Ok(value::to_map(&Output { c: a + b })?)
        }
        .boxed()
    }
}

#[actix::test]
async fn test_serve_iroh() {
    let tracker = TrackFlowRun::init_tracing_once();
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
    let tracker = TrackFlowRun::init_tracing_once();
    let client = command_factory::new_client(CommandFactory::collect(), tracker);
    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    dbg!("bind");
    let addr = endpoint.node_addr().initialized().await.unwrap();
    dbg!(&addr);
    client.bind_iroh(endpoint);

    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let client = command_factory::connect_iroh(endpoint, addr).await.unwrap();

    dbg!("connected");

    let nd = Add.node_data();

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
