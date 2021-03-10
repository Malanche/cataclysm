extern crate cataclysm;

use cataclysm::{Server, Path, http::{Response, Method}};

async fn hello_world() -> Response {
    Response::ok().body("hola mundo")
}

#[tokio::main]
async fn main() {
    let server = Server::new().path(
        Path::new("/hello").such_that(Method::Get.replies_with(hello_world))
    );

    server.run().await.unwrap();
}