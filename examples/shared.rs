use cataclysm::{Server, Branch, Shared, http::{Response, Method, Path}};
use std::sync::{Mutex};

use misc::SimpleLogger;
mod misc;

async fn index(path: Path<(String, i64)>, shared: Shared<Mutex<i64>>) -> Response {
    let path = path.into_inner();
    match shared.into_inner().lock() {
        Ok(mut counter) => {
            match path.0.as_str() {
                "add" => {
                    *counter += path.1;
                },
                "substract" => {
                    *counter -= path.1;
                },
                _ => return Response::internal_server_error()
            };
            Response::ok().body(format!("Counter at :{}!", counter))
        },
        Err(e) => panic!("{}", e)
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    // We create our tree structure
    let branch = Branch::new("/{regex:^(add|substract)$}/{:value}").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).share(Mutex::new(0)).build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}