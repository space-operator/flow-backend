use crate::command_side::command_factory::{self, CommandFactoryExt};
use crate::flow_side::remote_command::RemoteCommand;
use crate::tracing::TrackFlowRun;
use flow_lib::command::CommandFactory;
use flow_lib::command::CommandTrait;
use flow_lib::context::{CommandContext, execute};
use iroh::{Endpoint, Watcher};

#[actix::test]
async fn test_call() {
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
    let real_node = crate::error_node::build().unwrap();
    let node = RemoteCommand::new(client.init(&real_node.node_data()).await.unwrap().unwrap())
        .await
        .unwrap();
    let error = node
        .run(
            CommandContext::test_context(),
            flow_lib::value::map! { "x" => 0 },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error.downcast::<execute::Error>().unwrap(),
        execute::Error::Collected
    ));
}
