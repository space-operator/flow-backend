use super::prelude::*;
use crate::db_worker::token_worker::LoginWithAdminCred;
use chrono::{DateTime, Utc};
use db::local_storage::Jwt;
use flow_lib::config::Endpoints;

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub user_id: UserId,
    pub access_token: String,
    pub refresh_token: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expires_at: DateTime<Utc>,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/claim_token")
        .wrap(config.cors())
        .app_data(web::Data::new(config.endpoints()))
        .route(web::post().to(claim_token))
}

async fn claim_token(
    user: Auth<auth_v1::ApiKey>,
    db: web::Data<DbPool>,
    endpoints: web::Data<Endpoints>,
) -> Result<web::Json<Output>, Error> {
    let result = LoginWithAdminCred {
        client: reqwest::Client::new(),
        user_id: *user.user_id(),
        db: (**db).clone(),
        endpoints: (**endpoints).clone(),
    }
    .claim()
    .await;

    match result {
        Ok(Jwt {
            access_token,
            refresh_token,
            expires_at,
        }) => Ok(web::Json(Output {
            user_id: *user.user_id(),
            access_token,
            refresh_token,
            expires_at,
        })),
        Err(error) => Err(Error::custom(StatusCode::INTERNAL_SERVER_ERROR, error)),
    }
}
