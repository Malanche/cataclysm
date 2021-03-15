extern crate cataclysm;

use cataclysm::{Server, Path, http::{Response, Method}, SimpleLogger};

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
        Path::new("/hello").with(Method::Get.to(hello))
            .nested(Path::new("/world")
                .with(Method::Post.to(world))
        )
    ).build();

    server.run("localhost:8000").await.unwrap();
}