use futures::future::FutureExt;
use cataclysm::{Server, Branch, Additional, Pipeline, http::{Response, Request, Method}, SimpleLogger};
use std::sync::Arc;

async fn hello() -> Response {
    log::info!("hello callback called!");
    Response::ok().body("hello")
}

async fn world() -> Response {
    log::info!("world callback called!");
    Response::ok().body("world!")
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 10)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    
    let branch = Branch::new("/")
        .with(Method::Get.to(hello))
        .with(Method::Post.to(world))
        .layer(|req: Request, pipeline: Box<Pipeline<()>>, ad: Arc<Additional<()>>| async {
            // Example of timing layer
            log::info!("Time measuring begins");
            let now = std::time::Instant::now();
            let request = pipeline.execute(req, ad).await;
            let elapsed = now.elapsed().as_nanos();
            log::info!("Process time: {} ns", elapsed);
            request
        }.boxed());

    let server = Server::builder(
        branch
    ).build();

    server.run("127.0.0.1:8000").await.unwrap();
}