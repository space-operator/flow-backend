use super::super::prelude::*;
use value::Value;

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/read_item")
        .wrap(config.cors())
        .route(web::post().to(read_item))
}

#[derive(Deserialize)]
struct Params {
    store: String,
    key: String,
}

#[derive(Serialize)]
struct Output {
    value: Value,
}

async fn read_item(
    params: web::Json<Params>,
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<DbPool>,
) -> Result<web::Json<Output>, Error> {
    let opt = db
        .get_user_conn(*user.user_id())
        .await?
        .read_item(&params.store, &params.key)
        .await?;
    match opt {
        Some(value) => Ok(web::Json(Output { value })),
        None => Err(Error::custom(StatusCode::NOT_FOUND, "not found")),
    }
}
