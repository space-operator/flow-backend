use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error as ThisError;
use uuid::Uuid;

pub mod nft_metadata;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct ResultBool<T, E = PDGError>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    pub bSuccess: bool,
    #[serde(flatten)]
    pub success: Option<T>,
    #[serde(flatten)]
    pub error: Option<E>,
}

impl<T, E> From<Result<T, E>> for ResultBool<T, E>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(value) => Self {
                bSuccess: true,
                success: Some(value),
                error: None,
            },
            Err(error) => Self {
                bSuccess: false,
                success: None,
                error: Some(error),
            },
        }
    }
}

#[derive(ThisError, Debug)]
#[error("invalid result: {:?}", .0)]
pub struct Malformed<T: std::fmt::Debug>(pub T);

impl<T: std::fmt::Debug, E: std::fmt::Debug> ResultBool<T, E> {
    pub fn into_result(self) -> Result<Result<T, E>, Malformed<Self>> {
        if self.bSuccess
            && let Some(success) = self.success
        {
            Ok(Ok(success))
        } else if !self.bSuccess
            && let Some(error) = self.error
        {
            Ok(Err(error))
        } else {
            Err(Malformed(self))
        }
    }
}

impl<E: std::fmt::Debug + Serialize> ResultBool<(), E> {
    pub fn error_text(e: E) -> String {
        serde_json::to_string(&ResultBool::<(), E> {
            bSuccess: false,
            success: None,
            error: Some(e),
        })
        .unwrap_or_else(|error| {
            serde_json::to_string(&ResultBool::<(), PDGError> {
                bSuccess: false,
                success: None,
                error: Some(PDGError {
                    error: "SerializeError".to_owned(),
                    errorDetails: Some(error.to_string()),
                }),
            })
            .unwrap()
        })
    }
}

impl<T: std::fmt::Debug + Serialize> ResultBool<T, ()> {
    pub fn success_text(t: T) -> String {
        serde_json::to_string(&ResultBool::<T, ()> {
            bSuccess: true,
            success: Some(t),
            error: None,
        })
        .unwrap_or_else(|error| {
            serde_json::to_string(&ResultBool::<(), PDGError> {
                bSuccess: false,
                success: None,
                error: Some(PDGError {
                    error: "SerializeError".to_owned(),
                    errorDetails: Some(error.to_string()),
                }),
            })
            .unwrap()
        })
    }
}

#[derive(Serialize, Deserialize, ThisError, Debug, Clone)]
#[allow(non_snake_case)]
#[error("{:?} {}", error, errorDetails.as_ref().map(String::as_str).unwrap_or_default())]
pub struct PDGError {
    pub error: String,
    pub errorDetails: Option<String>,
}

/// Success reply from PDG POST request
#[derive(Serialize, Deserialize, Debug)]
pub struct PostReply {
    pub request_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RenderSuccess {
    pub request_uuid: Uuid,
    pub main_image_url: String,
    pub sketch_image_url: String,
    pub metadata_url: String,
}

/// The request clients send to `WsRender`.
#[derive(Serialize, Deserialize, Debug)]
pub struct RenderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rand_seed: Option<String>,
    pub version: String,
    #[serde(default)]
    pub workitem: WorkItem,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct WorkItem {
    pub attributes: HashMap<String, serde_json::Value>,
    pub batchIndex: i32,
    pub batchParentId: i32,
    pub cloneTargetId: i32,
    pub cookType: i32,
    pub customData: String,
    pub customDataType: String,
    pub executionType: i32,
    pub frame: i32,
    pub frameStep: i32,
    pub hasFrame: bool,
    pub id: i32,
    pub index: i32,
    pub isCloneResultData: bool,
    pub isFrozen: bool,
    pub isNoGenerate: bool,
    pub isPostCook: bool,
    pub isStatic: bool,
    pub loopBeginStackIds: Vec<i32>,
    pub loopBeginStackIters: Vec<i32>,
    pub loopBeginStackNumbers: Vec<i32>,
    pub loopBeginStackSizes: Vec<i32>,
    pub nodeName: String,
    pub priority: i32,
    pub state: i32,
    pub request_type: i32,
}

impl Default for WorkItem {
    fn default() -> Self {
        Self {
            attributes: <_>::default(),
            batchIndex: -1,
            batchParentId: -1,
            cloneTargetId: -1,
            cookType: 0,
            customData: "".to_owned(),
            customDataType: "genericdata".to_owned(),
            executionType: 0,
            frame: 0,
            frameStep: 1,
            hasFrame: false,
            id: 0,
            index: 0,
            isCloneResultData: false,
            isFrozen: false,
            isNoGenerate: false,
            isPostCook: false,
            isStatic: false,
            loopBeginStackIds: Vec::new(),
            loopBeginStackIters: Vec::new(),
            loopBeginStackNumbers: Vec::new(),
            loopBeginStackSizes: Vec::new(),
            nodeName: "csvoutput2".to_owned(),
            priority: 0,
            state: 5,
            request_type: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attribute {
    pub concat: bool,
    pub flag: u32,
    pub own: bool,
    pub r#type: u32,
    pub value: Vec<serde_json::Value>,
}

/// The request clients send to `WsWait`.
#[derive(Serialize, Deserialize)]
pub struct WaitRequest {
    pub request_uuid: Uuid,
}
