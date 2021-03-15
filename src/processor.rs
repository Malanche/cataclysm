use crate::http::{Response};
#[async_trait::async_trait]
pub trait Processor {
    async fn handle(&self) -> Response;
}

#[async_trait::async_trait]
impl<T, F> Processor for T where T: Fn() -> F + Sync, F: std::future::Future<Output = Response> + std::marker::Send {
    async fn handle(&self) -> Response {
        self().await
    }
}