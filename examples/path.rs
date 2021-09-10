use cataclysm::{Server, Branch, http::{Response, Method, Path}};

async fn index(path: Path<(String, i32)>) -> Response {
    let path = path.into_inner();
    Response::ok().body(format!("Hello {}, your id is {}!", path.0, path.1))
}

#[tokio::main]
async fn main() {
    // We create our tree structure
    let branch = Branch::new("/{:username}/{:id}").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch).build();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}