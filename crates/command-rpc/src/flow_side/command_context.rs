use std::time::Duration;

use anyhow::Context;
use bincode::config::standard;
use capnp::capability::Promise;
use flow_lib::{
    UserId,
    context::{CommandContext, execute, get_jwt, signer},
    flow_run_events::NodeLogContent,
    solana::Pubkey,
    utils::tower_client::CommonErrorExt,
    value,
};
use futures::{TryFutureExt, future::LocalBoxFuture};
use tower::{Service, ServiceExt};

use crate::anyhow2capnp;
pub use crate::command_capnp::command_context::*;

impl tower::Service<signer::SignatureRequest> for Client {
    type Response = signer::SignatureResponse;

    type Error = signer::Error;

    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: signer::SignatureRequest) -> Self::Future {
        let client = self.clone();
        Box::pin(async move {
            let mut request = client.request_signature_request();
            request.get().set_request(
                &bincode::encode_to_vec(
                    RequestSignatureData {
                        pubkey: req.pubkey,
                        message: req.message,
                        timeout: req.timeout,
                    },
                    standard(),
                )
                .map_err(signer::Error::other)?,
            );
            let resp = request.send().promise.await.map_err(signer::Error::other)?;
            let resp: signer::SignatureResponse = bincode::decode_from_slice(
                resp.get()
                    .map_err(signer::Error::other)?
                    .get_response()
                    .map_err(signer::Error::other)?,
                standard(),
            )
            .map_err(signer::Error::other)?
            .0;

            Ok(resp)
        })
    }
}

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

impl tower::Service<get_jwt::Request> for Client {
    type Response = get_jwt::Response;

    type Error = get_jwt::Error;

    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: get_jwt::Request) -> Self::Future {
        let client = self.clone();
        Box::pin(
            async move {
                let mut request = client.get_jwt_request();
                request.get().set_user_id(req.user_id.to_string());
                let response = request.send().promise.await.context("send")?;
                let access_token = response
                    .get()
                    .context("get")?
                    .get_access_token()
                    .context("get_access_token")?
                    .to_str()
                    .context("utf8")?;
                Ok::<_, anyhow::Error>(get_jwt::Response {
                    access_token: access_token.to_owned(),
                })
            }
            .map_err(get_jwt::Error::from_anyhow),
        )
    }
}

pub struct CommandContextImpl {
    pub(crate) context: CommandContext,
}

pub struct RequestSignatureData {
    pub pubkey: Pubkey,
    pub message: bytes::Bytes,
    pub timeout: Duration,
}

impl bincode::Encode for RequestSignatureData {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.pubkey.as_array().encode(encoder)?;
        self.message.as_ref().encode(encoder)?;
        bincode::serde::Compat(&self.timeout).encode(encoder)?;
        Ok(())
    }
}

impl<C> bincode::Decode<C> for RequestSignatureData {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let pubkey = Pubkey::new_from_array(<[u8; 32]>::decode(decoder)?);
        let message = bytes::Bytes::from(Vec::<u8>::decode(decoder)?);
        let timeout = bincode::serde::Compat::<Duration>::decode(decoder)?.0;
        Ok(Self {
            pubkey,
            message,
            timeout,
        })
    }
}

impl<'de, C> bincode::BorrowDecode<'de, C> for RequestSignatureData {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = C>>(
        d: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        bincode::Decode::<C>::decode(d)
    }
}

pub type RequestSignatureResponse = signer::SignatureResponse;

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
            let data = params
                .get()
                .context("get")?
                .get_request()
                .context("get_request")?;
            let request = bincode::decode_from_slice::<execute::Request, _>(data, standard())
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
                &bincode::encode_to_vec(response, standard())
                    .context("encode execute::Response")?,
            );
            Ok(())
        }
    }

    fn get_jwt_impl(
        &mut self,
        params: GetJwtParams,
        mut results: GetJwtResults,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + 'static {
        let svc = self.context.raw().services.get_jwt.clone();
        async move {
            let user_id: UserId = params
                .get()
                .context("get")?
                .get_user_id()
                .context("get_user_id")?
                .to_str()
                .context("utf8")?
                .parse()
                .context("parse user_id")?;
            let token = svc
                .ready_oneshot()
                .await
                .context("ready")?
                .call(get_jwt::Request { user_id })
                .await?
                .access_token;
            results.get().set_access_token(token);
            Ok(())
        }
    }

    fn logs_impl(&mut self, params: LogParams, _: LogResults) -> Result<(), anyhow::Error> {
        let data = params.get()?.get_log()?;
        let log: NodeLogContent = bincode::decode_from_slice(data, standard())?.0;
        self.context.log(log)?;
        Ok(())
    }

    fn request_signature_impl(
        &mut self,
        params: RequestSignatureParams,
        mut results: RequestSignatureResults,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + 'static {
        let mut ctx = self.context.clone();
        async move {
            let data = bincode::decode_from_slice::<RequestSignatureData, _>(
                params.get()?.get_request()?,
                standard(),
            )?
            .0;
            let result = ctx
                .request_signature(data.pubkey, data.message, data.timeout)
                .await?;
            results
                .get()
                .set_response(&bincode::encode_to_vec(result, standard())?);
            Ok(())
        }
    }
}

impl Server for CommandContextImpl {
    fn data(&mut self, params: DataParams, result: DataResults) -> Promise<(), capnp::Error> {
        self.data_impl(params, result).map_err(anyhow2capnp).into()
    }

    fn execute(
        &mut self,
        params: ExecuteParams,
        result: ExecuteResults,
    ) -> Promise<(), capnp::Error> {
        Promise::from_future(self.execute_impl(params, result).map_err(anyhow2capnp))
    }

    fn get_jwt(
        &mut self,
        params: GetJwtParams,
        results: GetJwtResults,
    ) -> Promise<(), capnp::Error> {
        Promise::from_future(self.get_jwt_impl(params, results).map_err(anyhow2capnp))
    }

    fn log(&mut self, params: LogParams, results: LogResults) -> Promise<(), capnp::Error> {
        self.logs_impl(params, results).map_err(anyhow2capnp).into()
    }

    fn request_signature(
        &mut self,
        params: RequestSignatureParams,
        results: RequestSignatureResults,
    ) -> Promise<(), capnp::Error> {
        Promise::from_future(
            self.request_signature_impl(params, results)
                .map_err(anyhow2capnp),
        )
    }
}
