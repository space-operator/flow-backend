use serde::{Serialize, Deserialize}
use hashbrown::HashMap;
use flow_lib::Value;

pub mod start_flow {
    use super::*;
    #[derive(Deserialize)]
    pub struct Params {
        #[serde(default)]
        pub inputs: HashMap<String, Value>,
        #[serde(default)]
        pub partial_config: Option<PartialConfig>,
        #[serde(default)]
        pub environment: HashMap<String, String>,
    }

    #[derive(Serialize)]
    pub struct Output {
        pub flow_run_id: FlowRunId,
    }
}
