use cataclysm::{Server, Branch, http::{Response, Method, Path}};

use misc::SimpleLogger;
mod misc;

async fn index(path: Path<(String, i32)>) -> Response {
    Response::ok().body(format!("Hello {}, your id is {}!", (*path).0, (*path).1))
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/{:username}/{:id}").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}