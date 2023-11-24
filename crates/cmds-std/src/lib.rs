use flow_lib::command::CommandError;

pub mod const_cmd;
pub mod flow_run_info;
pub mod json_extract;
pub mod json_insert;
pub mod kvstore;
pub mod note;
pub mod print_cmd;
pub mod storage;
pub mod wait_cmd;
pub mod postgrest;

#[derive(serde::Deserialize)]
pub struct ErrorBody {
    pub error: String,
}

pub async fn supabase_error(code: reqwest::StatusCode, resp: reqwest::Response) -> CommandError {
    let bytes = resp.bytes().await.unwrap_or_default();
    match serde_json::from_slice::<ErrorBody>(&bytes) {
        Ok(ErrorBody { error }) => CommandError::msg(error),
        _ => {
            let body = String::from_utf8_lossy(&bytes);
            anyhow::anyhow!("{}: {}", code, body)
        }
    }
}
