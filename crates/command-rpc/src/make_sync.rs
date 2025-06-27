use std::fmt::Debug;

use futures::{
    FutureExt, StreamExt,
    channel::{mpsc, oneshot},
    ready,
};
use pin_project_lite::pin_project;
use tokio::task::spawn_local;
use tower::{Service, ServiceExt};

struct Message<T, U, E> {
    req: T,
    sender: oneshot::Sender<Result<U, E>>,
}

pub struct MakeSync<T, U, E> {
    sender: mpsc::Sender<Message<T, U, E>>,
}

impl<T, U, E> Clone for MakeSync<T, U, E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T, U, E> MakeSync<T, U, E>
where
    T: 'static,
    U: 'static,
    E: Debug + 'static,
{
    pub fn new<S>(mut service: S) -> Self
    where
        S: Service<T, Response = U, Error = E> + 'static,
    {
        let (sender, mut receiver) = mpsc::channel(0);
        spawn_local(async move {
            while let Some(Message { req, sender }) = receiver.next().await {
                match service.ready().await {
                    Ok(service) => {
                        let future = service.call(req);
                        spawn_local(async move {
                            let result = future.await;
                            sender.send(result).ok();
                        });
                    }
                    Err(error) => {
                        sender.send(Err(error)).ok();
                    }
                }
            }
        });
        Self { sender }
    }
}

impl<T, U, E> Service<T> for MakeSync<T, U, E> {
    type Response = U;

    type Error = E;

    type Future = CallFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        match ready!(self.sender.poll_ready(cx)) {
            Ok(()) => std::task::Poll::Ready(Ok(())),
            Err(_) => panic!("we don't close receiver manually"),
        }
    }

    fn call(&mut self, req: T) -> Self::Future {
        let (sender, receiver) = oneshot::channel();
        if let Err(error) = self.sender.try_send(Message { req, sender }) {
            if error.is_full() {
                panic!("poll_ready must be called first");
            } else if error.is_disconnected() {
                unreachable!("we don't close receiver manually");
            } else {
                panic!("unknown error: {error}");
            }
        }
        CallFuture { receiver }
    }
}

pin_project! {
    pub struct CallFuture<T> {
        receiver: oneshot::Receiver<T>,
    }
}

impl<T> Future for CallFuture<T> {
    type Output = T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match ready!(self.project().receiver.poll_unpin(cx)) {
            Ok(result) => std::task::Poll::Ready(result),
            Err(_) => {
                unreachable!("we always send a result");
            }
        }
    }
}
