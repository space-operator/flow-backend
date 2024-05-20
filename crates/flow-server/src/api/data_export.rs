use super::prelude::*;
use db::connection::ExportedUserData;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/export")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(export))
}

async fn export(
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<DbPool>,
) -> Result<web::Json<ExportedUserData>, Error> {
    let user_id = user.into_inner().user_id;
    let data = db.get_user_conn(user_id).await?.export_user_data().await?;
    Ok(web::Json(data))
}
