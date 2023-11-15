use super::*;
use crate::FlowRunLogsRow;
use flow_lib::{context::get_jwt, BoxError, UserId};
use reqwest::header::AUTHORIZATION;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use thiserror::Error as ThisError;
use value::ConstBytes;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("failed to get JWT: {}", .0)]
    Jwt(#[from] get_jwt::Error),
    #[error("{}", .0)]
    Upstream(String),
}

pub struct ProxiedUserConn {
    pub user_id: UserId,
    pub client: reqwest::Client,
    pub rpc_url: String,
    pub push_logs_url: String,
    pub jwt_svc: get_jwt::Svc,
}

#[derive(Serialize, Deserialize)]
pub struct RpcRequest<'a, T: Serialize> {
    method: &'a str,
    params: T,
}

impl ProxiedUserConn {
    pub async fn push_logs(&self, rows: &[FlowRunLogsRow]) -> crate::Result<()> {
        let jwt = self
            .jwt_svc
            .call_ref(get_jwt::Request {
                user_id: self.user_id,
            })
            .await
            .map_err(Error::from)?
            .access_token;
        self.client
            .post(self.push_logs_url.clone())
            .header(AUTHORIZATION, &format!("Bearer {}", jwt))
            .json(&rows)
            .send()
            .await
            .map_err(Error::from)?
            .error_for_status()
            .map_err(Error::from)?;
        Ok(())
    }

    async fn send<P, T>(&self, method: &str, params: &P) -> crate::Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let jwt = self
            .jwt_svc
            .call_ref(get_jwt::Request {
                user_id: self.user_id,
            })
            .await
            .map_err(Error::from)?
            .access_token;
        let result = self
            .client
            .post(self.rpc_url.clone())
            .header(AUTHORIZATION, &format!("Bearer {}", jwt))
            .json(&RpcRequest { method, params })
            .send()
            .await
            .map_err(Error::from)?
            .json::<Result<T, String>>()
            .await
            .map_err(Error::from)?;
        Ok(result.map_err(Error::Upstream)?)
    }
}

#[async_trait::async_trait]
impl UserConnectionTrait for ProxiedUserConn {
    async fn get_flow_owner(&self, flow_id: FlowId) -> crate::Result<UserId> {
        self.send("get_flow_owner", &(flow_id,)).await
    }

    async fn get_wallets(&self) -> crate::Result<Vec<Wallet>> {
        self.send::<[(); 0], _>("get_wallets", &[]).await
    }

    async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        self.send("clone_flow", &(flow_id,)).await
    }

    async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
    ) -> crate::Result<FlowRunId> {
        self.send("new_flow_run", &(&config, &inputs)).await
    }

    async fn get_previous_values(
        &self,
        nodes: &HashMap<NodeId, FlowRunId>,
    ) -> crate::Result<HashMap<NodeId, Vec<Value>>> {
        self.send("get_previous_values", &(nodes,)).await
    }

    async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig> {
        self.send("get_flow_config", &(id,)).await
    }

    async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()> {
        self.send("set_start_time", &(&id, &time)).await
    }

    async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()> {
        self.send("push_flow_error", &(&id, &error)).await
    }

    async fn push_flow_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()> {
        self.send(
            "push_flow_log",
            &(&id, &index, &time, &level, &module, &content),
        )
        .await
    }

    async fn set_run_result(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
        not_run: &[NodeId],
        output: &Value,
    ) -> crate::Result<()> {
        self.send("set_run_result", &(&id, &time, &not_run, &output))
            .await
    }

    async fn new_node_run(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        input: &Value,
    ) -> crate::Result<()> {
        self.send("new_node_run", &(&id, &node_id, &times, &time, &input))
            .await
    }

    async fn save_node_output(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        output: &Value,
    ) -> crate::Result<()> {
        self.send("save_node_output", &(&id, &node_id, &times, &output))
            .await
    }

    async fn push_node_error(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        error: &str,
    ) -> crate::Result<()> {
        self.send("push_node_error", &(&id, &node_id, &times, &error))
            .await
    }

    async fn push_node_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()> {
        self.send(
            "push_node_log",
            &(
                &id, &index, &node_id, &times, &time, &level, &module, &content,
            ),
        )
        .await
    }

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
        self.send("set_node_finish", &(&id, &node_id, &times, &time))
            .await
    }

    async fn new_signature_request(&self, pubkey: &[u8; 32], message: &[u8]) -> crate::Result<i64> {
        self.send(
            "new_signature_request",
            &(&Value::from(*pubkey), &Value::from(message)),
        )
        .await
    }

    async fn save_signature(&self, id: &i64, signature: &[u8; 64]) -> crate::Result<()> {
        self.send("save_signature", &(&id, &Value::from(*signature)))
            .await
    }

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>> {
        self.send("read_item", &(&store, &key)).await
    }
}

impl UserConnection {
    pub async fn process_rpc(&mut self, req_json: &str) -> Result<Box<RawValue>, BoxError> {
        let req: RpcRequest<'_, &'_ RawValue> = serde_json::from_str(req_json)?;
        match req.method {
            "get_flow_owner" => {
                let (id,) = serde_json::from_str(req.params.get())?;
                let res = self.get_flow_owner(id).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "get_wallets" => {
                let res = self.get_wallets().await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "clone_flow" => {
                let (flow_id,) = serde_json::from_str(req.params.get())?;
                let res = self.clone_flow(flow_id).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "new_flow_run" => {
                let (config, inputs) = serde_json::from_str(req.params.get())?;
                let res = self.new_flow_run(&config, &inputs).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "get_previous_values" => {
                let (nodes,) = serde_json::from_str(req.params.get())?;
                let res = self.get_previous_values(&nodes).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "get_flow_config" => {
                let (id,) = serde_json::from_str(req.params.get())?;
                let res = self.get_flow_config(id).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "set_start_time" => {
                let (id, time) = serde_json::from_str(req.params.get())?;
                let res = self.set_start_time(&id, &time).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "push_flow_error" => {
                let (id, error): (_, String) = serde_json::from_str(req.params.get())?;
                let res = self.push_flow_error(&id, &error).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "push_flow_log" => {
                let (id, index, time, level, module, content): (_, _, _, String, _, String) =
                    serde_json::from_str(req.params.get())?;
                let res = self
                    .push_flow_log(&id, &index, &time, &level, &module, &content)
                    .await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "set_run_result" => {
                let (id, time, not_run, output): (_, _, Vec<NodeId>, _) =
                    serde_json::from_str(req.params.get())?;
                let res = self.set_run_result(&id, &time, &not_run, &output).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "new_node_run" => {
                let (id, node_id, times, time, input) = serde_json::from_str(req.params.get())?;
                let res = self
                    .new_node_run(&id, &node_id, &times, &time, &input)
                    .await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "save_node_output" => {
                let (id, node_id, times, output) = serde_json::from_str(req.params.get())?;
                let res = self
                    .save_node_output(&id, &node_id, &times, &output)
                    .await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "push_node_error" => {
                let (id, node_id, times, error): (_, _, _, String) =
                    serde_json::from_str(req.params.get())?;
                let res = self.push_node_error(&id, &node_id, &times, &error).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "set_node_finish" => {
                let (id, node_id, times, time) = serde_json::from_str(req.params.get())?;
                let res = self.set_node_finish(&id, &node_id, &times, &time).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "new_signature_request" => {
                let (pubkey, message): (Value, Value) = serde_json::from_str(req.params.get())?;
                let pubkey = value::from_value::<ConstBytes<32>>(pubkey)?.0;
                let message = value::from_value::<serde_bytes::ByteBuf>(message)?.into_vec();
                let res = self.new_signature_request(&pubkey, &message).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "save_signature" => {
                let (id, signature): (_, Value) = serde_json::from_str(req.params.get())?;
                let signature = value::from_value::<ConstBytes<64>>(signature)?.0;
                let res = self.save_signature(&id, &signature).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            "read_item" => {
                let (store, key): (String, String) = serde_json::from_str(req.params.get())?;
                let res = self.read_item(&store, &key).await?;
                Ok(serde_json::value::to_raw_value(&res)?)
            }
            name => Err(format!("unknown method: {}", name).into()),
        }
    }
}
