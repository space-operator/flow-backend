use super::prelude::*;
use db::connection::ExportedUserData;

pub fn service(config: &Config) -> Option<impl HttpServiceFactory> {
    Some(
        web::resource("/import")
            .wrap(config.service_key()?)
            .wrap(config.cors())
            .route(web::post().to(import)),
    )
}

async fn import(
    db: web::Data<RealDbPool>,
    data: web::Json<ExportedUserData>,
) -> Result<web::Json<Success>, Error> {
    let data = data.into_inner();
    db.get_admin_conn().await?.import_data(data).await?;
    Ok(web::Json(Success))
}
