use actix_web::{
    Error, ResponseError,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::{
    TryFutureExt,
    future::{Either, MapOk},
};
use std::{
    future::{Ready, ready},
    ops::Deref,
    rc::Rc,
};

pub fn rc_fn_ref<F, E>(f: F) -> ReqFn<Rc<dyn Function>>
where
    F: for<'r> Fn(&'r ServiceRequest) -> Result<(), E> + 'static,
    E: ResponseError + 'static,
{
    ReqFn {
        f: Rc::new(FnRef(f)),
    }
}

pub trait Function: 'static {
    fn call(&self, req: ServiceRequest) -> Result<ServiceRequest, ServiceResponse>;
}

struct FnRef<F>(F);

impl<F, E> Function for FnRef<F>
where
    F: for<'r> Fn(&'r ServiceRequest) -> Result<(), E> + 'static,
    E: ResponseError + 'static,
{
    fn call(&self, mut req: ServiceRequest) -> Result<ServiceRequest, ServiceResponse> {
        match (self.0)(&mut req) {
            Ok(_) => Ok(req),
            Err(e) => Err(req.error_response(e)),
        }
    }
}

pub struct ReqFn<C> {
    f: C,
}

impl<C, F, S, B> Transform<S, ServiceRequest> for ReqFn<C>
where
    C: Deref<Target = F> + Clone + 'static,
    F: Function + ?Sized,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Transform = ReqFnMiddleware<C, S>;
    type Response = <Self::Transform as Service<ServiceRequest>>::Response;
    type Error = <Self::Transform as Service<ServiceRequest>>::Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();

    fn new_transform(&self, s: S) -> Self::Future {
        ready(Ok(ReqFnMiddleware {
            s,
            f: self.f.clone(),
        }))
    }
}

pub struct ReqFnMiddleware<C, S> {
    f: C,
    s: S,
}

impl<C, F, S, B> Service<ServiceRequest> for ReqFnMiddleware<C, S>
where
    C: Deref<Target = F> + Clone + 'static,
    F: Function + ?Sized,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Either<
        MapOk<S::Future, fn(ServiceResponse<B>) -> Self::Response>,
        Ready<Result<Self::Response, Self::Error>>,
    >;

    forward_ready!(s);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        match self.f.deref().call(req) {
            Ok(req) => Either::Left(
                self.s
                    .call(req)
                    .map_ok(ServiceResponse::<B>::map_into_left_body),
            ),
            Err(resp) => Either::Right(ready(Ok(resp.map_into_right_body()))),
        }
    }
}
