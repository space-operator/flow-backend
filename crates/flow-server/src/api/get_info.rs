use super::prelude::*;
use crate::db_worker::{GetIrohInfo, IrohInfo};
use actix::SystemService;
use actix_web::{HttpResponseBuilder, dev::ConnectionInfo, http::header::ContentType};
use url::Url;

#[derive(Serialize)]
struct Output {
    supabase_url: Url,
    anon_key: String,
    iroh: IrohInfo,
    base_url: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    let supabase_url = config.supabase.endpoint.url.clone();
    let anon_key = config.supabase.anon_key.clone();
    web::resource("/info").route(web::get().to(move |info: ConnectionInfo| {
        let supabase_url = supabase_url.clone();
        let anon_key = anon_key.clone();

        async move {
            let db_worker = DBWorker::from_registry();
            let iroh = db_worker.send(GetIrohInfo).await.unwrap().unwrap();
            let base_url = format!("{}://{}", info.scheme(), info.host());
            let output = Output {
                supabase_url,
                anon_key,
                iroh,
                base_url,
            };
            let json = simd_json::to_vec(&output).unwrap();
            HttpResponseBuilder::new(StatusCode::OK)
                .insert_header(ContentType::json())
                .body(json)
        }
    }))
}
