use super::read_item::SuccessBody;
use crate::supabase_error;
use flow_lib::command::prelude::*;
use futures_util::future::join_all;
use reqwest::{StatusCode, header::AUTHORIZATION};

pub const KV_EXPLORER: &str = "kvexplorer";

const VALUES: &str = "Values";
const KVS: &str = "KVS";

#[derive(Serialize, Debug)]
struct ItemKey {
    store: String,
    key: String,
}

#[derive(Debug)]
pub struct ExplorerCommand {
    outputs: Vec<Output>,
    pinned: Vec<ItemKey>,
}

impl ExplorerCommand {
    fn new(data: &NodeData) -> Result<Self, CommandError> {
        let outputs = data
            .sources
            .iter()
            .map(|o| Output {
                name: o.name.clone(),
                r#type: o.r#type.clone(),
                optional: false,
            })
            .collect();
        let pinned = serde_json::from_value::<Vec<String>>(
            data.targets_form
                .extra
                .rest
                .get("pinned")
                .cloned()
                .unwrap_or_default(),
        )?
        .into_iter()
        .filter_map(|s| {
            let idx = s.find('/')?;
            if idx + 1 >= s.len() {
                return None;
            }
            Some(ItemKey {
                store: s[..idx].to_owned(),
                key: s[idx + 1..].to_owned(),
            })
        })
        .collect();
        Ok(Self { outputs, pinned })
    }
}

async fn read_item(
    client: &reqwest::Client,
    url: &str,
    key: &ItemKey,
    auth: &str,
) -> Result<Value, CommandError> {
    let mut req = client.post(url).json(&key);
    req = req.header(AUTHORIZATION, auth);

    let resp = req.send().await?;

    let code = resp.status();
    if code == StatusCode::OK {
        Ok(resp.json::<SuccessBody>().await?.value)
    } else {
        Err(supabase_error(code, resp).await)
    }
}

#[async_trait]
impl CommandTrait for ExplorerCommand {
    fn name(&self) -> Name {
        KV_EXPLORER.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [].to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        self.outputs.clone()
    }

    fn permissions(&self) -> Permissions {
        Permissions { user_tokens: true }
    }

    async fn run(&self, mut ctx: CommandContextX, _: ValueSet) -> Result<ValueSet, CommandError> {
        let auth = ctx.get_jwt_header().await?;
        let url = format!("{}/kv/read_item", ctx.endpoints().flow_server);
        let results = join_all(
            self.pinned
                .iter()
                .map(|k| read_item(&ctx.http(), &url, k, &auth)),
        )
        .await;

        let push_values = self.outputs.iter().any(|o| o.name == VALUES);
        let push_kvs = self.outputs.iter().any(|o| o.name == KVS);
        let mut values = Vec::new();
        let mut kvs = Vec::new();
        let mut output = value::Map::new();
        self.pinned
            .iter()
            .zip(results)
            .for_each(|(k, result)| match result {
                Ok(value) => {
                    if push_values {
                        values.push(value.clone());
                    }
                    if push_kvs {
                        kvs.push(Value::from(value::map! {
                            "store" => k.store.clone(),
                            "key" => k.key.clone(),
                        }));
                    }
                    let name = format!("{}/{}", k.store, k.key);
                    if self.outputs.iter().any(|o| o.name == name) {
                        output.insert(name, value);
                    }
                }
                Err(error) => {
                    tracing::error!("failed to get {:?}: {}", k, error);
                }
            });

        if push_values {
            output.insert(VALUES.to_owned(), Value::Array(values));
        }
        if push_kvs {
            output.insert(KVS.to_owned(), Value::Array(kvs));
        }
        Ok(output)
    }
}

flow_lib::submit!(CommandDescription::new(KV_EXPLORER, |data: &NodeData| {
    Ok(Box::new(ExplorerCommand::new(data)?))
}));
