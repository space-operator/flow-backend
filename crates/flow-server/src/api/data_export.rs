use super::prelude::*;
use db::connection::ExportedUserData;

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/export")
        .wrap(config.cors())
        .route(web::post().to(export))
}

async fn export(
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<DbPool>,
) -> Result<web::Json<ExportedUserData>, Error> {
    let data = db
        .get_user_conn(*user.user_id())
        .await?
        .export_user_data()
        .await?;
    Ok(web::Json(data))
}
