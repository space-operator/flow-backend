use super::super::prelude::*;
use value::Value;

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/write_item")
        .wrap(config.cors())
        .route(web::post().to(write_item))
}

#[derive(Deserialize)]
struct Params {
    store: String,
    key: String,
    value: Value,
}

#[derive(Serialize)]
struct Output {
    old_value: Option<Value>,
}

fn parse_error(e: DbError) -> Error {
    if let DbError::Execute { error, context, .. } = &e {
        let name = error.as_db_error().and_then(|e| e.constraint());
        if name == Some("kvstore_user_id_store_name_fkey") {
            return Error::custom(StatusCode::NOT_FOUND, "store not found");
        }

        if *context == "update user_quotas" {
            return Error::custom(StatusCode::FORBIDDEN, "user's storage limit exceeded");
        }
    }
    e.into()
}

async fn write_item(
    params: web::Json<Params>,
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Output>, Error> {
    let old_value = db
        .get_admin_conn()
        .await?
        .insert_or_replace_item(user.user_id(), &params.store, &params.key, &params.value)
        .await
        .map_err(parse_error)?;
    Ok(web::Json(Output { old_value }))
}
