use crate::command_side::command_factory::{self, CommandFactoryExt};
use crate::flow_side::command_context::CommandContextImpl;
use crate::tracing::TrackFlowRun;
use flow_lib::{
    command::CommandFactory,
    context::CommandContext,
    value::{self, bincode_impl::map_from_bincode, bincode_impl::map_to_bincode},
};
use iroh::{Endpoint, Watcher};
use rust_decimal_macros::dec;

#[actix::test]
async fn test_serve_iroh() {
    let tracker = TrackFlowRun::init_tracing_once();
    let (addr, availables) = {
        let factory = CommandFactory::collect();
        let availables = factory.availables().collect::<Vec<_>>();
        let factory = command_factory::new_client(factory, tracker);
        let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
        let addr = endpoint.node_addr().initialized().await.unwrap();
        factory.bind_iroh(endpoint);
        (addr, availables)
    };

    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let client = command_factory::connect_iroh(endpoint, addr).await.unwrap();
    let names = client.all_availables().await.unwrap();
    assert_eq!(names, availables);
    assert!(!names.is_empty());
}

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

    let nd = crate::add::build().unwrap().node_data();

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
