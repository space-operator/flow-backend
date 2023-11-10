use super::prelude::*;
use db::{
    apikey::{self, NameConflict},
    Error as DbError,
};

#[derive(Deserialize)]
pub struct Params {
    name: String,
}

#[derive(Serialize)]
pub struct Output {
    pub full_key: String,
    #[serde(flatten)]
    pub key: apikey::APIKey,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/create")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(create_key))
}

async fn create_key(
    params: web::Json<Params>,
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Output>, Error> {
    let user_id = user.into_inner().user_id;
    let Params { name } = params.into_inner();
    let r = db.get_user_conn(user_id).await?.create_apikey(&name).await;
    let (key, full_key) = match r {
        Ok(r) => r,
        Err(DbError::LogicError(NameConflict)) => {
            return Err(Error::custom(StatusCode::BAD_REQUEST, "NameConflict"))
        }
        Err(error) => return Err(error.erase_type().into()),
    };
    Ok(web::Json(Output { full_key, key }))
}
