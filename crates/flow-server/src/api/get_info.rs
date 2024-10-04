use super::prelude::*;
use actix_web::{http::header::ContentType, HttpResponseBuilder};
use url::Url;

#[derive(Serialize)]
struct Output {
    supabase_url: Url,
    anon_key: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory {
    let output = Output {
        supabase_url: config.supabase.endpoint.url.clone(),
        anon_key: config.supabase.anon_key.clone(),
    };
    let json: bytes::Bytes = serde_json::to_vec(&output).unwrap().into();
    web::resource("/info").route(web::get().to(move || {
        std::future::ready(
            HttpResponseBuilder::new(StatusCode::OK)
                .insert_header(ContentType::json())
                .body(json.clone()),
        )
    }))
}
