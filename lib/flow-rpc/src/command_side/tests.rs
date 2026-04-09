use crate::command_side::command_factory::{self, CommandFactoryExt};
use crate::command_side::command_trait;
use crate::flow_side::command_context::CommandContextImpl;
use crate::flow_side::remote_command::RemoteCommand;
use crate::tracing::TrackFlowRun;
use bytes::Bytes;
use flow_lib::{
    CmdOutputDescription, Name, ValueType,
    command::{CommandFactory, CommandTrait},
    context::{CommandContext, FlowServices, FlowSetServices, execute, get_jwt, signer},
    flow_run_events,
    solana::{Pubkey, Signature},
    utils::tower_client::unimplemented_svc,
    value::{self, bincode_impl::map_from_bincode, bincode_impl::map_to_bincode},
};
use iroh::{Endpoint, Watcher};
use rust_decimal_macros::dec;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

fn test_context(execute_svc: execute::Svc, signer_svc: signer::Svc) -> CommandContext {
    let base = CommandContext::test_context();
    let data = base.raw().data.clone();
    let node_id = data.node_id;
    let times = data.times;
    let (tx, _) = flow_run_events::channel();

    CommandContext::builder()
        .execute(execute_svc)
        .get_jwt(unimplemented_svc::<
            get_jwt::Request,
            get_jwt::Response,
            get_jwt::Error,
        >())
        .flow(FlowServices {
            signer: signer_svc,
            set: FlowSetServices {
                http: base.http().clone(),
                solana_client: base.solana_client().clone(),
                helius: None,
                extensions: Default::default(),
                api_input: unimplemented_svc(),
            },
        })
        .data(data)
        .node_log(flow_run_events::NodeLogSender::new(tx, node_id, times))
        .build()
}

struct RequestSignatureCommand {
    pubkey: Pubkey,
}

#[async_trait::async_trait(?Send)]
impl CommandTrait for RequestSignatureCommand {
    fn name(&self) -> Name {
        "request_signature_test".into()
    }

    fn inputs(&self) -> Vec<flow_lib::CmdInputDescription> {
        Vec::new()
    }

    fn outputs(&self) -> Vec<CmdOutputDescription> {
        vec![CmdOutputDescription {
            name: "sig_len".into(),
            r#type: ValueType::U64,
            optional: false,
        }]
    }

    async fn run(
        &self,
        mut ctx: CommandContext,
        _params: value::Map,
    ) -> Result<value::Map, anyhow::Error> {
        let response = ctx
            .request_signature(
                self.pubkey,
                None,
                Bytes::from_static(b"rpc-signature-test"),
                Duration::from_secs(12),
            )
            .await?;
        Ok(value::map! {
            "sig_len" => response.signature.as_array().len() as u64,
        })
    }
}

#[actix::test]
async fn test_serve_iroh() {
    let tracker = TrackFlowRun::init_tracing_once();
    let (addr, availables) = {
        let factory = CommandFactory::collect();
        let availables = factory.availables().collect::<Vec<_>>();
        let factory = command_factory::new_client(factory, tracker);
        let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
        let addr = endpoint.node_addr().initialized().await;
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
async fn test_call_add() {
    let tracker = TrackFlowRun::init_tracing_once();
    let client = command_factory::new_client(CommandFactory::collect(), tracker);
    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let addr = endpoint.node_addr().initialized().await;
    client.bind_iroh(endpoint);

    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let client = command_factory::connect_iroh(endpoint, addr).await.unwrap();

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

#[actix::test]
async fn test_call_error() {
    let tracker = TrackFlowRun::init_tracing_once();
    let client = command_factory::new_client(CommandFactory::collect(), tracker);
    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let addr = endpoint.node_addr().initialized().await;
    client.bind_iroh(endpoint);

    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let client = command_factory::connect_iroh(endpoint, addr).await.unwrap();

    let nd = crate::error_node::build().unwrap().node_data();

    let cmd = client.init(&nd).await.unwrap().unwrap();

    let mut req = cmd.run_request();
    req.get().set_inputs(
        map_to_bincode(&value::map! {
            "x" => 0,
        })
        .unwrap()
        .as_slice(),
    );
    req.get().set_ctx(capnp_rpc::new_client(CommandContextImpl {
        context: CommandContext::test_context(),
    }));
    let Err(error) = req.send().promise.await else {
        panic!();
    };
    println!("{:?}", error);
}

#[actix::test]
async fn test_command_trait_wires_signer_service() {
    let tracker = TrackFlowRun::init_tracing_once();
    let pubkey = Pubkey::new_unique();
    let observed = Arc::new(Mutex::new(None::<signer::SignatureRequest>));
    let signer_svc = signer::Svc::new(tower::service_fn({
        let observed = observed.clone();
        move |req: signer::SignatureRequest| {
            let observed = observed.clone();
            async move {
                *observed.lock().unwrap() = Some(req);
                Ok(signer::SignatureResponse {
                    signature: Signature::from([9u8; 64]),
                    new_message: None,
                })
            }
        }
    }));

    let cmd = RemoteCommand::new(command_trait::new_client(
        Box::new(RequestSignatureCommand { pubkey }),
        tracker,
    ))
    .await
    .unwrap();

    let output = cmd
        .run(
            test_context(
                unimplemented_svc::<execute::Request, execute::Response, execute::Error>(),
                signer_svc,
            ),
            value::map! {},
        )
        .await
        .unwrap();

    assert_eq!(
        value::from_value::<u64>(output["sig_len"].clone()).unwrap(),
        64
    );

    let request = observed.lock().unwrap().clone().unwrap();
    assert_eq!(request.pubkey, pubkey);
    assert_eq!(request.message, Bytes::from_static(b"rpc-signature-test"));
    assert_eq!(request.timeout, Duration::from_secs(12));
}
