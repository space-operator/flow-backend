use super::prelude::*;
use crate::db_worker::{GetProviders, ProviderInfo};
use actix::SystemService;
use actix_web::{HttpResponseBuilder, http::header::ContentType};

#[derive(Serialize)]
struct Output {
    providers: Vec<ProviderInfo>,
}

pub fn service() -> impl HttpServiceFactory + 'static {
    web::resource("/providers").route(web::get().to(|| async {
        let db_worker = DBWorker::from_registry();
        let providers = db_worker.send(GetProviders).await.unwrap().unwrap_or_default();
        let output = Output { providers };
        let json = simd_json::to_vec(&output).unwrap();
        HttpResponseBuilder::new(StatusCode::OK)
            .insert_header(ContentType::json())
            .body(json)
    }))
}