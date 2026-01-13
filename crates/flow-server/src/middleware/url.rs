use std::convert::Infallible;

use actix::fut::Ready;
use actix_web::{
    FromRequest,
    web::{self, ServiceConfig},
};

pub struct ServerBaseUrl(pub String);

impl FromRequest for ServerBaseUrl {
    type Error = Infallible;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let info = req.connection_info();
        let host = info.host().to_string();
        let state = req
            .app_data::<web::ThinData<State>>()
            .expect("server must be configured, call configure()");
        let host = if state.allowed.contains(&host) {
            host
        } else {
            state.default.clone()
        };
        let scheme = match info.scheme() {
            "ws" => "http",
            "wss" => "https",
            scheme => scheme,
        };
        actix::fut::ready(Ok(Self(format!("{}://{}", scheme, host))))
    }
}

struct State {
    allowed: Vec<String>,
    default: String,
}

pub fn configure(cfg: &mut ServiceConfig, server_config: &crate::Config) {
    cfg.app_data(web::ThinData(State {
        allowed: server_config.allowed_hostnames.clone(),
        default: server_config.server_hostname.clone(),
    }));
}
