use super::super::prelude::*;
use db::SqlState;
use once_cell::sync::Lazy;
use regex::Regex;

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/create_store")
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

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^[a-zA-Z][a-zA-Z0-9_-]*$"#).unwrap());
    if !RE.is_match(s) {
        return Err(StoreNameError::WrongFormat { regex: RE.as_str() });
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
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<DbPool>,
) -> Result<web::Json<Success>, Error> {
    let params = params.into_inner();
    check_store_name(&params.store).map_err(|e| Error::custom(StatusCode::BAD_REQUEST, e))?;
    db.get_admin_conn()
        .await?
        .create_store(user.user_id(), &params.store)
        .await
        .map_err(process_error)?;
    Ok(web::Json(Success))
}
