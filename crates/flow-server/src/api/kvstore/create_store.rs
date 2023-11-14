use super::super::prelude::*;
use db::SqlState;
use once_cell::sync::Lazy;
use regex::Regex;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/create_store")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(create_store))
}

#[derive(Deserialize)]
struct Params {
    store: String,
}

#[derive(ThisError, Debug)]
enum StoreNameError {
    #[error("max store name length is {}", .max)]
    MaxLength { max: usize },
    #[error("store name must match the regex: '{}'", .regex)]
    WrongFormat { regex: &'static str },
}

fn check_store_name(s: &str) -> Result<(), StoreNameError> {
    const MAX_LEN: usize = 120;
    if s.len() > MAX_LEN {
        return Err(StoreNameError::MaxLength { max: MAX_LEN });
    }

    const RE_STR: &str = r#"^[a-zA-Z][a-zA-Z0-9_-]*$"#;
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(RE_STR).unwrap());
    if !RE.is_match(s) {
        return Err(StoreNameError::WrongFormat { regex: RE_STR });
    }
    Ok(())
}

fn process_error(e: DbError) -> Error {
    if let DbError::Execute { error, context, .. } = &e {
        if *context == "insert kvstore_metadata"
            && error.code() == Some(&SqlState::UNIQUE_VIOLATION)
        {
            return Error::custom(StatusCode::PRECONDITION_FAILED, "database already exists");
        }

        if *context == "update user_quotas"
            && error.to_string().contains("unexpected number of rows")
        {
            return Error::custom(StatusCode::FORBIDDEN, "user's storage limit exceeded");
        }
    }

    e.into()
}

async fn create_store(
    params: web::Json<Params>,
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Success>, Error> {
    let params = params.into_inner();
    check_store_name(&params.store).map_err(|e| Error::custom(StatusCode::BAD_REQUEST, e))?;
    db.get_admin_conn()
        .await?
        .create_store(&user.user_id, &params.store)
        .await
        .map_err(process_error)?;
    Ok(web::Json(Success))
}
