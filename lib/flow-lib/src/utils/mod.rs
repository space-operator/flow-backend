use std::{future::Future, pin::Pin};

pub mod extensions;
pub mod tower_client;

pub use extensions::Extensions;
pub use tower_client::TowerClient;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
