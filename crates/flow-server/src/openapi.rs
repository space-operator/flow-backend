use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use utoipa::{OpenApi, ToSchema};

#[derive(Serialize, Deserialize, ToSchema)]
struct SuccessDoc {
    success: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct IrohInfoDoc {
    node_id: String,
    relay_url: String,
    direct_addresses: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ServiceInfoDoc {
    supabase_url: String,
    anon_key: String,
    iroh: IrohInfoDoc,
    base_url: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct AuthInitParamsDoc {
    pubkey: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct AuthInitOutputDoc {
    msg: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ConfirmAuthParamsDoc {
    token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ConfirmAuthOutputDoc {
    session: JsonValue,
    new_user: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ClaimTokenOutputDoc {
    user_id: String,
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ValuesConfigDoc {
    nodes: std::collections::BTreeMap<String, String>,
    default_run_id: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct PartialConfigDoc {
    only_nodes: Vec<String>,
    values_config: ValuesConfigDoc,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct SolanaActionConfigDoc {
    action_signer: String,
    action_identity: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
enum IValueDoc {
    String {
        S: String,
    },
    Decimal {
        D: String,
    },
    I64 {
        I: String,
    },
    U64 {
        U: String,
    },
    I128 {
        I1: String,
    },
    U128 {
        U1: String,
    },
    Float {
        F: String,
    },
    Bool {
        B: bool,
    },
    Null {
        N: i32,
    },
    Pubkey {
        B3: String,
    },
    Keypair {
        B6: String,
    },
    Bytes {
        BY: String,
    },
    Array {
        #[schema(no_recursion)]
        A: Vec<IValueDoc>,
    },
    Map {
        #[schema(no_recursion)]
        M: BTreeMap<String, IValueDoc>,
    },
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
enum JsonValueDocNonNull {
    String(String),
    Number(f64),
    Bool(bool),
    #[schema(no_recursion)]
    Array(Vec<JsonValueDoc>),
    #[schema(no_recursion)]
    Object(BTreeMap<String, JsonValueDoc>),
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
struct JsonValueDoc(Option<JsonValueDocNonNull>);

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
enum FlowInputValueDoc {
    Typed(IValueDoc),
    Json(JsonValueDoc),
}

#[derive(Serialize, Deserialize, ToSchema)]
struct StartFlowParamsDoc {
    inputs: Option<BTreeMap<String, FlowInputValueDoc>>,
    partial_config: Option<PartialConfigDoc>,
    environment: Option<BTreeMap<String, String>>,
    output_instructions: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct StartFlowSharedParamsDoc {
    inputs: Option<BTreeMap<String, FlowInputValueDoc>>,
    output_instructions: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct StartFlowUnverifiedParamsDoc {
    inputs: Option<BTreeMap<String, FlowInputValueDoc>>,
    output_instructions: Option<bool>,
    action_identity: Option<String>,
    action_config: Option<SolanaActionConfigDoc>,
    fees: Option<Vec<(String, f64)>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct StopFlowParamsDoc {
    timeout_millies: Option<u64>,
    reason: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct FlowRunStartOutputDoc {
    flow_run_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct FlowRunTokenOutputDoc {
    flow_run_id: String,
    token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct CloneFlowOutputDoc {
    flow_id: String,
    id_map: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct StartDeploymentParamsDoc {
    inputs: Option<BTreeMap<String, FlowInputValueDoc>>,
    action_signer: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ReadFlowParamsDoc {
    inputs: Option<BTreeMap<String, FlowInputValueDoc>>,
    skip_cache: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ReadDeploymentParamsDoc {
    inputs: Option<BTreeMap<String, FlowInputValueDoc>>,
    skip_cache: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct CreateApiKeyParamsDoc {
    name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ApiKeyRecordDoc {
    key_hash: String,
    trimmed_key: String,
    name: String,
    user_id: String,
    created_at: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct CreateApiKeyOutputDoc {
    full_key: String,
    #[serde(flatten)]
    key: ApiKeyRecordDoc,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct DeleteApiKeyParamsDoc {
    key_hash: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ApiKeyInfoOutputDoc {
    user_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct KvStoreParamsDoc {
    store: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct KvReadParamsDoc {
    store: String,
    key: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct KvWriteParamsDoc {
    store: String,
    key: String,
    value: JsonValue,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct KvWriteOutputDoc {
    old_value: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct KvReadOutputDoc {
    value: JsonValue,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct KvDeleteOutputDoc {
    old_value: JsonValue,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct SubmitSignatureDoc {
    id: i64,
    signature: String,
    new_msg: Option<String>,
}

#[utoipa::path(
    get,
    path = "/info",
    tag = "service",
    responses((status = 200, description = "Service info", body = ServiceInfoDoc))
)]
fn get_info_doc() {}

#[utoipa::path(
    get,
    path = "/healthcheck",
    tag = "service",
    responses((status = 200, description = "Healthcheck", body = SuccessDoc))
)]
fn healthcheck_doc() {}

#[utoipa::path(
    post,
    path = "/auth/init",
    tag = "auth",
    request_body = AuthInitParamsDoc,
    responses((status = 200, description = "Initialize wallet auth", body = AuthInitOutputDoc))
)]
fn init_auth_doc() {}

#[utoipa::path(
    post,
    path = "/auth/confirm",
    tag = "auth",
    request_body = ConfirmAuthParamsDoc,
    responses((status = 200, description = "Confirm wallet auth", body = ConfirmAuthOutputDoc))
)]
fn confirm_auth_doc() {}

#[utoipa::path(
    post,
    path = "/auth/claim_token",
    tag = "auth",
    responses((status = 200, description = "Claim bearer token from API key", body = ClaimTokenOutputDoc))
)]
fn claim_token_doc() {}

#[utoipa::path(
    post,
    path = "/flow/start/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = Option<StartFlowParamsDoc>,
    responses((status = 200, description = "Start owned or shared flow", body = FlowRunStartOutputDoc))
)]
fn start_flow_doc() {}

#[utoipa::path(
    post,
    path = "/flow/start_shared/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = Option<StartFlowSharedParamsDoc>,
    responses((status = 200, description = "Start shared flow", body = FlowRunStartOutputDoc))
)]
fn start_flow_shared_doc() {}

#[utoipa::path(
    post,
    path = "/flow/start_unverified/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = Option<StartFlowUnverifiedParamsDoc>,
    responses((status = 200, description = "Start unverified flow", body = FlowRunTokenOutputDoc))
)]
fn start_flow_unverified_doc() {}

#[utoipa::path(
    get,
    path = "/flow/read/{id}",
    tag = "flows",
    params(
        ("id" = String, Path, description = "Flow id"),
        ("inputs" = Option<String>, Query, description = "JSON-encoded typed or plain flow inputs"),
        ("skip_cache" = Option<bool>, Query, description = "Bypass server read cache for this request")
    ),
    responses(
        (status = 200, description = "Read owned or adapter-authorized flow", body = IValueDoc),
        (status = 304, description = "Not modified"),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_flow_doc() {}

#[utoipa::path(
    post,
    path = "/flow/read/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = Option<ReadFlowParamsDoc>,
    responses(
        (status = 200, description = "Read owned or adapter-authorized flow", body = IValueDoc),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_flow_post_doc() {}

#[utoipa::path(
    get,
    path = "/flow/read_shared/{id}",
    tag = "flows",
    params(
        ("id" = String, Path, description = "Flow id"),
        ("inputs" = Option<String>, Query, description = "JSON-encoded typed or plain flow inputs"),
        ("skip_cache" = Option<bool>, Query, description = "Bypass server read cache for this request")
    ),
    responses(
        (status = 200, description = "Read shared flow", body = IValueDoc),
        (status = 304, description = "Not modified"),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_flow_shared_doc() {}

#[utoipa::path(
    post,
    path = "/flow/read_shared/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = Option<ReadFlowParamsDoc>,
    responses(
        (status = 200, description = "Read shared flow", body = IValueDoc),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_flow_shared_post_doc() {}

#[utoipa::path(
    get,
    path = "/flow/read_unverified/{id}",
    tag = "flows",
    params(
        ("id" = String, Path, description = "Flow id"),
        ("inputs" = Option<String>, Query, description = "JSON-encoded typed or plain flow inputs"),
        ("skip_cache" = Option<bool>, Query, description = "Bypass server read cache for this request")
    ),
    responses(
        (status = 200, description = "Read unverified flow", body = IValueDoc),
        (status = 304, description = "Not modified"),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_flow_unverified_doc() {}

#[utoipa::path(
    post,
    path = "/flow/read_unverified/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = Option<ReadFlowParamsDoc>,
    responses(
        (status = 200, description = "Read unverified flow", body = IValueDoc),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_flow_unverified_post_doc() {}

#[utoipa::path(
    post,
    path = "/flow/stop/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow run id")),
    request_body = Option<StopFlowParamsDoc>,
    responses((status = 200, description = "Stop flow", body = SuccessDoc))
)]
fn stop_flow_doc() {}

#[utoipa::path(
    post,
    path = "/flow/clone/{id}",
    tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    responses((status = 200, description = "Clone flow", body = CloneFlowOutputDoc))
)]
fn clone_flow_doc() {}

#[utoipa::path(
    post,
    path = "/deployment/start",
    tag = "deployments",
    params(
        ("id" = Option<String>, Query, description = "Deployment id"),
        ("flow" = Option<String>, Query, description = "Flow id"),
        ("tag" = Option<String>, Query, description = "Deployment tag")
    ),
    request_body = Option<StartDeploymentParamsDoc>,
    responses((status = 200, description = "Start deployment", body = FlowRunTokenOutputDoc))
)]
fn start_deployment_doc() {}

#[utoipa::path(
    get,
    path = "/deployment/read",
    tag = "deployments",
    params(
        ("id" = Option<String>, Query, description = "Deployment id"),
        ("flow" = Option<String>, Query, description = "Flow id"),
        ("tag" = Option<String>, Query, description = "Deployment tag"),
        ("inputs" = Option<String>, Query, description = "JSON-encoded typed or plain flow inputs"),
        ("skip_cache" = Option<bool>, Query, description = "Bypass server read cache for this request")
    ),
    responses(
        (status = 200, description = "Read deployment", body = IValueDoc),
        (status = 304, description = "Not modified"),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_deployment_doc() {}

#[utoipa::path(
    post,
    path = "/deployment/read",
    tag = "deployments",
    params(
        ("id" = Option<String>, Query, description = "Deployment id"),
        ("flow" = Option<String>, Query, description = "Flow id"),
        ("tag" = Option<String>, Query, description = "Deployment tag")
    ),
    request_body = Option<ReadDeploymentParamsDoc>,
    responses(
        (status = 200, description = "Read deployment", body = IValueDoc),
        (status = 403, description = "Read not allowed"),
        (status = 408, description = "Read timed out")
    )
)]
fn read_deployment_post_doc() {}

#[utoipa::path(
    post,
    path = "/signature/submit",
    tag = "signatures",
    request_body = SubmitSignatureDoc,
    responses((status = 200, description = "Submit signature", body = SuccessDoc))
)]
fn submit_signature_doc() {}

#[utoipa::path(
    post,
    path = "/apikey/create",
    tag = "apiKeys",
    request_body = CreateApiKeyParamsDoc,
    responses((status = 200, description = "Create API key", body = CreateApiKeyOutputDoc))
)]
fn create_apikey_doc() {}

#[utoipa::path(
    post,
    path = "/apikey/delete",
    tag = "apiKeys",
    request_body = DeleteApiKeyParamsDoc,
    responses((status = 200, description = "Delete API key", body = Value))
)]
fn delete_apikey_doc() {}

#[utoipa::path(
    get,
    path = "/apikey/info",
    tag = "apiKeys",
    responses((status = 200, description = "API key owner info", body = ApiKeyInfoOutputDoc))
)]
fn apikey_info_doc() {}

#[utoipa::path(
    post,
    path = "/kv/create_store",
    tag = "kv",
    request_body = KvStoreParamsDoc,
    responses((status = 200, description = "Create KV store", body = SuccessDoc))
)]
fn kv_create_store_doc() {}

#[utoipa::path(
    post,
    path = "/kv/delete_store",
    tag = "kv",
    request_body = KvStoreParamsDoc,
    responses((status = 200, description = "Delete KV store", body = SuccessDoc))
)]
fn kv_delete_store_doc() {}

#[utoipa::path(
    post,
    path = "/kv/write_item",
    tag = "kv",
    request_body = KvWriteParamsDoc,
    responses((status = 200, description = "Write KV item", body = KvWriteOutputDoc))
)]
fn kv_write_item_doc() {}

#[utoipa::path(
    post,
    path = "/kv/read_item",
    tag = "kv",
    request_body = KvReadParamsDoc,
    responses((status = 200, description = "Read KV item", body = KvReadOutputDoc))
)]
fn kv_read_item_doc() {}

#[utoipa::path(
    post,
    path = "/kv/delete_item",
    tag = "kv",
    request_body = KvReadParamsDoc,
    responses((status = 200, description = "Delete KV item", body = KvDeleteOutputDoc))
)]
fn kv_delete_item_doc() {}

#[utoipa::path(
    post,
    path = "/wallets/upsert",
    tag = "wallets",
    request_body = Value,
    responses((status = 200, description = "Upsert wallet", body = Value))
)]
fn wallet_upsert_doc() {}

#[utoipa::path(
    post,
    path = "/data/export",
    tag = "data",
    responses((status = 200, description = "Export user data", body = Value))
)]
fn data_export_doc() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_info_doc,
        healthcheck_doc,
        init_auth_doc,
        confirm_auth_doc,
        claim_token_doc,
        start_flow_doc,
        start_flow_shared_doc,
        start_flow_unverified_doc,
        read_flow_doc,
        read_flow_post_doc,
        read_flow_shared_doc,
        read_flow_shared_post_doc,
        read_flow_unverified_doc,
        read_flow_unverified_post_doc,
        stop_flow_doc,
        clone_flow_doc,
        start_deployment_doc,
        read_deployment_doc,
        read_deployment_post_doc,
        submit_signature_doc,
        create_apikey_doc,
        delete_apikey_doc,
        apikey_info_doc,
        kv_create_store_doc,
        kv_delete_store_doc,
        kv_write_item_doc,
        kv_read_item_doc,
        kv_delete_item_doc,
        wallet_upsert_doc,
        data_export_doc
    ),
    components(
        schemas(
            SuccessDoc,
            IrohInfoDoc,
            ServiceInfoDoc,
            AuthInitParamsDoc,
            AuthInitOutputDoc,
            ConfirmAuthParamsDoc,
            ConfirmAuthOutputDoc,
            ClaimTokenOutputDoc,
            ValuesConfigDoc,
            PartialConfigDoc,
            SolanaActionConfigDoc,
            StartFlowParamsDoc,
            StartFlowSharedParamsDoc,
            StartFlowUnverifiedParamsDoc,
            ReadFlowParamsDoc,
            StopFlowParamsDoc,
            FlowRunStartOutputDoc,
            FlowRunTokenOutputDoc,
            CloneFlowOutputDoc,
            StartDeploymentParamsDoc,
            ReadDeploymentParamsDoc,
            CreateApiKeyParamsDoc,
            ApiKeyRecordDoc,
            CreateApiKeyOutputDoc,
            DeleteApiKeyParamsDoc,
            ApiKeyInfoOutputDoc,
            KvStoreParamsDoc,
            KvReadParamsDoc,
            KvWriteParamsDoc,
            KvWriteOutputDoc,
            KvReadOutputDoc,
            KvDeleteOutputDoc,
            SubmitSignatureDoc
        )
    ),
    tags(
        (name = "service", description = "Service metadata and health"),
        (name = "auth", description = "Authentication bootstrap and token exchange"),
        (name = "flows", description = "Flow execution and cloning"),
        (name = "deployments", description = "Deployment execution"),
        (name = "signatures", description = "Signature submission"),
        (name = "apiKeys", description = "API key management"),
        (name = "kv", description = "Key-value store operations"),
        (name = "wallets", description = "Wallet management"),
        (name = "data", description = "User data export")
    )
)]
pub struct FlowServerOpenApiDoc;
