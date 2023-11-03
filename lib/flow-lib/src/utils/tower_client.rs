use std::{error::Error as StdError, future::Future, task::Poll};
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};

use super::BoxFuture;

pub struct TowerClient<T, U, E> {
    inner: Buffer<BoxService<T, U, E>, T>,
    worker_error: fn(tower::BoxError) -> E,
}

impl<T, U, E> Clone for TowerClient<T, U, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            worker_error: self.worker_error,
        }
    }
}

impl<T, U, E> TowerClient<T, U, E>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Into<tower::BoxError> + StdError + Send + Sync + 'static,
{
    pub fn new(
        inner: Buffer<BoxService<T, U, E>, T>,
        worker_error: fn(tower::BoxError) -> E,
    ) -> Self {
        Self {
            inner,
            worker_error,
        }
    }

    pub fn from_service<S>(s: S, worker_error: fn(tower::BoxError) -> E, size: usize) -> Self
    where
        S: tower::Service<T, Response = U, Error = E> + Send + 'static,
        S::Future: Send + 'static,
    {
        let buffer = Buffer::new(BoxService::new(s), size);
        Self::new(buffer, worker_error)
    }

    pub async fn call_mut(&mut self, req: T) -> Result<U, E> {
        self.ready().await?.call(req).await
    }

    pub async fn call_ref(&self, req: T) -> Result<U, E> {
        let mut this: Self = (*self).clone();
        this.call_mut(req).await
    }
}

impl<T, U, E> TowerClient<T, U, E>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Into<tower::BoxError> + StdError + Send + Sync + 'static,
{
    pub fn unimplemented<F>(error: F, worker_error: fn(tower::BoxError) -> E) -> Self
    where
        F: Fn() -> E + Send + 'static,
    {
        let s = tower::ServiceBuilder::new()
            .boxed()
            .service_fn(move |_: T| {
                let error = error();
                async move { Err(error) }
            });

        let (buffer, worker) = tower::buffer::Buffer::pair(s, 32);
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(worker);
        }
        TowerClient::new(buffer, worker_error)
    }
}

impl<T, U, E> tower::Service<T> for TowerClient<T, U, E>
where
    E: Into<tower::BoxError> + StdError + Send + Sync + 'static,
{
    type Response = U;
    type Error = E;
    type Future = ResponseFuture<U, E>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(self.worker_error)
    }

    fn call(&mut self, req: T) -> Self::Future {
        let fut = self.inner.call(req);
        ResponseFuture {
            fut,
            worker_error: self.worker_error,
        }
    }
}

pin_project_lite::pin_project! {
    pub struct ResponseFuture<U, E> {
        #[pin]
        fut: tower::buffer::future::ResponseFuture<BoxFuture<'static, Result<U, E>>>,
        worker_error: fn(tower::BoxError) -> E,
    }
}

impl<U, E> Future for ResponseFuture<U, E>
where
    E: Into<tower::BoxError> + StdError + Send + Sync + 'static,
{
    type Output = Result<U, E>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let result = std::task::ready!(self.as_mut().project().fut.poll(cx));
        Poll::Ready(match result {
            Ok(resp) => Ok(resp),
            Err(boxed) => match boxed.downcast::<E>() {
                Ok(error) => Err(*error),
                Err(boxed) => Err((self.worker_error)(boxed)),
            },
        })
    }
}
