use cataclysm::{Server, Branch, http::{Response, Method, Query}};
use serde::Deserialize;

use misc::SimpleLogger;
mod misc;

#[derive(Deserialize)]
struct QueryParams {
    name: String
}

async fn index(query: Query<QueryParams>) -> Response {
    Response::ok().body(format!("Hello, {}!", query.into_inner().name))
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Trace).init().unwrap();
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/attempt").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}