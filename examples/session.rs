use cataclysm::{Server, Branch, session::{Session, CookieSession}, http::{Response, Method, Path}};

use misc::SimpleLogger;
mod misc;

async fn index(session: Session) -> Response {
    match session.get("username") {
        Some(username) => {
            log::info!("processing request for user {}", username);
            let message = format!("Hello, {}", username);
            session.apply(Response::ok().body(message))
        },
        None => {
            log::info!("rejecting request with empty session");
            Response::unauthorized()
        }
    }
}

async fn login(path: Path<(String,)>, mut session: Session) -> Response {
    let (username, ) = path.into_inner();
    log::info!("creating cookie session for user {}", username);
    session.set("username", username);
    session.apply(Response::ok())
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index))
        .merge(Branch::new("/login/{:username}").with(Method::Get.to(login)));
    // We create a server with the given tree structure
    let server = Server::builder(branch).session_creator(CookieSession::new()).build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}