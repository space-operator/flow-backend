use std::convert::Infallible;

use actix::fut::Ready;
use actix_web::FromRequest;
use url::Url;

pub struct FullUrl(pub Url);

impl FromRequest for FullUrl {
    type Error = Infallible;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        actix::fut::ready(Ok(Self(req.full_url())))
    }
}
