pub mod auth_v1;
pub mod req_fn;
pub mod url;

pub fn optional<T, E: Into<actix_web::Error>>(
    x: Result<T, E>,
) -> Result<Option<T>, actix_web::Error> {
    match x {
        Ok(x) => Ok(Some(x)),
        Err(e) => {
            let e: actix_web::Error = e.into();
            let j = e.as_error::<actix_web::error::JsonPayloadError>();
            match j {
                Some(&actix_web::error::JsonPayloadError::ContentType) => Ok(None),
                _ => Err(e),
            }
        }
    }
}
