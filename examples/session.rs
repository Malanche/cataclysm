use cataclysm::{Server, Branch, Session, http::{Response, Method, Path}};

async fn index(session: Session) -> Response {
    match session.get("username") {
        Some(username) => {
            let message = format!("Hello, {}", username);
            session.apply(Response::ok().body(message))
        },
        None => Response::unauthorized()
    }
}

async fn login(path: Path<(String,)>, mut session: Session) -> Response {
    session.set("username", path.into_inner().0);
    session.apply(Response::ok())
}

#[tokio::main]
async fn main() {
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index))
        .merge(Branch::new("/login/{:username}").with(Method::Get.to(login)));
    // We create a server with the given tree structure
    let server = Server::builder(branch).build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}