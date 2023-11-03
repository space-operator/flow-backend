use flow_lib::UserId;
use std::path::PathBuf;

pub mod create_signed_url;
pub mod delete;
pub mod download;
pub mod explorer;
pub mod get_file_metadata;
pub mod get_public_url;
pub mod list;
pub mod upload;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum FileSpec {
    Key { key: String },
    BucketPath { bucket: String, path: PathBuf },
}

impl FileSpec {
    pub fn key(&self, user_id: &UserId) -> String {
        match self {
            FileSpec::Key { key } => key.clone(),
            FileSpec::BucketPath { bucket, path } => {
                if ["user-storages", "user-public-storages"].contains(&bucket.as_str()) {
                    format!("{}/{}/{}", bucket, user_id, path.display())
                } else {
                    format!("{}/{}", bucket, path.display())
                }
            }
        }
    }
}
