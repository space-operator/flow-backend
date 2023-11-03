use actix::MailboxError;
use futures_util::{future::Map, FutureExt};

#[derive(Clone)]
pub struct ActixService<T>
where
    T: actix::Message + Send,
    T::Result: Send,
{
    inner: actix::Recipient<T>,
}

impl<T> From<actix::Recipient<T>> for ActixService<T>
where
    T: actix::Message + Send,
    T::Result: Send,
{
    fn from(inner: actix::Recipient<T>) -> Self {
        Self { inner }
    }
}

fn convert_error<U, E>(result: Result<Result<U, E>, MailboxError>) -> Result<U, E>
where
    E: From<MailboxError>,
{
    match result {
        Ok(Ok(ok)) => Ok(ok),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(E::from(err)),
    }
}

impl<T, U, E> tower::Service<T> for ActixService<T>
where
    T: actix::Message<Result = Result<U, E>> + Send,
    U: Send,
    E: From<MailboxError> + Send,
{
    type Response = U;
    type Error = E;
    type Future = Map<
        actix::dev::RecipientRequest<T>,
        fn(Result<Result<U, E>, MailboxError>) -> Result<U, E>,
    >;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: T) -> Self::Future {
        self.inner.send(req).map(convert_error::<U, E>)
    }
}
