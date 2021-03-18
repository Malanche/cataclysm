use crate::{Extractor, http::{Response, Request}};

#[async_trait::async_trait]
pub trait Processor {
    async fn handle(&self, req: &Request) -> Response;
}

#[async_trait::async_trait]
impl<T, F> Processor for T where T: Fn() -> F + Sync, F: std::future::Future<Output = Response> + std::marker::Send {
    async fn handle(&self, _req: &Request) -> Response {
        self().await
    }
}

#[async_trait::async_trait]
impl<F, E> Processor for dyn Fn(E) -> F + Sync where
    E: Extractor,
    F: std::future::Future<Output = Response> + std::marker::Send {
    async fn handle(&self, req: &Request) -> Response {
        let value = <E as Extractor>::extract(req);
        self(value).await
    }
}