use anyhow::Context;
use bincode::config::standard;
use capnp::capability::Promise;
use flow_lib::{
    context::{CommandContext, execute},
    utils::tower_client::CommonErrorExt,
    value,
};
use futures::{TryFutureExt, future::LocalBoxFuture};
use tower::{Service, ServiceExt};

pub use crate::command_capnp::command_context::*;
use crate::r2p;

impl tower::Service<execute::Request> for Client {
    type Response = execute::Response;

    type Error = execute::Error;

    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: execute::Request) -> Self::Future {
        let client = self.clone();
        Box::pin(async move {
            let mut request = client.execute_request();
            request.get().set_request(
                &bincode::encode_to_vec(&req, standard()).map_err(execute::Error::other)?,
            );
            let resp = request
                .send()
                .promise
                .await
                .map_err(execute::Error::other)?;
            let resp: execute::Response = bincode::decode_from_slice(
                resp.get()
                    .context("get")
                    .map_err(execute::Error::from_anyhow)?
                    .get_response()
                    .context("get")
                    .map_err(execute::Error::from_anyhow)?,
                standard(),
            )
            .map_err(execute::Error::other)?
            .0;
            Ok(resp)
        })
    }
}

pub struct CommandContextImpl {
    pub(crate) context: CommandContext,
}

impl CommandContextImpl {
    fn data_impl(&mut self, _: DataParams, mut result: DataResults) -> Result<(), anyhow::Error> {
        let ctx_data = self.context.raw().data;
        let data = value::to_value(ctx_data)?.to_bincode()?;
        result.get().set_data(&data);
        Ok(())
    }

    fn execute_impl(
        &mut self,
        params: ExecuteParams,
        mut result: ExecuteResults,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + 'static {
        let svc = self.context.raw().services.execute.clone();
        async move {
            let request = bincode::decode_from_slice::<execute::Request, _>(
                params
                    .get()
                    .context("get")?
                    .get_request()
                    .context("get_request")?,
                standard(),
            )
            .context("decode execute::Request")?
            .0;
            let response = svc
                .ready_oneshot()
                .await
                .context("ready")?
                .call(request)
                .await
                .context("execute")?;

            result.get().set_response(
                &bincode::encode_to_vec(&response, standard())
                    .context("encode execute::Response")?,
            );
            Ok(())
        }
    }
}

impl Server for CommandContextImpl {
    fn data(&mut self, params: DataParams, result: DataResults) -> Promise<(), capnp::Error> {
        r2p(self
            .data_impl(params, result)
            .map_err(|error| capnp::Error::failed(error.to_string())))
    }

    fn execute(
        &mut self,
        params: ExecuteParams,
        result: ExecuteResults,
    ) -> Promise<(), capnp::Error> {
        Promise::from_future(
            self.execute_impl(params, result)
                .map_err(|error| capnp::Error::failed(error.to_string())),
        )
    }
}
