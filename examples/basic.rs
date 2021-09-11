use cataclysm::{Server, Branch, Shared, http::{Response, Method, Path}};

// Receives a string, and concatenates the shared suffix
async fn index(path: Path<(String,)>, shared: Shared<String>) -> Response {
    let (prefix,) = path.into_inner();
    let suffix = shared.into_inner();
    Response::ok().body(format!("{}{}", prefix, suffix))
}

#[tokio::main]
async fn main() {
    // We create our tree structure
    let branch = Branch::new("/{:value}").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).share("!!!".into()).build();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}
/*
use cataclysm::{Server, Branch, http::{Response, Method}};

async fn index() -> Response {
    Response::ok().body("hello")
}

#[tokio::main]
async fn main() {
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).build();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}
*/