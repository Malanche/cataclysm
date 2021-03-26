extern crate cataclysm;

use futures::future::FutureExt;
use cataclysm::{Server, Path, Pipeline, http::{Response, Request, Method}, SimpleLogger};

async fn hello() -> Response {
    Response::ok().body("hello")
}

async fn world() -> Response {
    Response::ok().body("world!")
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 10)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    
    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello)).middleware(|req: Request, pipeline: Box<Pipeline>| async {
            // Example of timing middleware
            let now = std::time::Instant::now();
            let request = match *pipeline {
                Pipeline::Layer(function, nested_pipeline) => function(req, nested_pipeline),
                Pipeline::Core(function) => function(req)
            }.await;
            let elapsed = now.elapsed().as_millis();
            log::info!("Process time: {}", elapsed);
            request
        }.boxed())
    ).build();

    /*
    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello)).with(Method::Post.to(world)).defaults_to(|| async {
            Response::ok().body("Perdido?")
        })
    ).build();


    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello))
            .nested(Path::new("/world")
                .with(Method::Get.to(world)))
            .nested(Path::new("/data")
                .with(Method::Post.to(|data: Vec<u8>| async {
                    log::info!("{}", String::from_utf8(data).unwrap());
                    Response::ok().body("haha")
                }))
        )
    ).build();
    */

    server.run("localhost:8000").await.unwrap();
}