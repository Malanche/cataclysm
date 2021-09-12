use cataclysm::{Server, Branch, http::{Response, Method}};

use misc::SimpleLogger;
mod misc;

async fn index() -> Response {
    Response::ok().body("hello")
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).log_format("[%M %P] %S, from %A").build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}