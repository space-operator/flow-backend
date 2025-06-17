use super::prelude::*;
use actix_web::{HttpResponseBuilder, http::header::ContentType};
use url::Url;

#[derive(Serialize)]
struct Output {
    supabase_url: Url,
    anon_key: String,
    iroh_node_id: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    let output = Output {
        supabase_url: config.supabase.endpoint.url.clone(),
        anon_key: config.supabase.anon_key.clone(),
        iroh_node_id: config.iroh_secret_key.public().to_string(),
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
