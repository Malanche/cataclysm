extern crate cataclysm;

use cataclysm::{Server, Path, http::{Response, Method}, SimpleLogger};

async fn hello_world() -> Response {
    Response::ok().body("hola mundo")
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 10)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    /*
    let server = Server::builder().path(
        Path::new("/hello").such_that(Method::Get.replies_with(hello_world))
    ).build();
    */
    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello_world))
            //.nested(Path::new("/world")
            //    .with(Method::Post.to(hello_world))
        //)
    ).build();

    server.run().await.unwrap();
}