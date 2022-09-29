use cataclysm::{Server, Branch, CorsBuilder, http::{Response, Method}};

/* To test the preflight cors response, execute in the terminal:
 *
 * `curl http://localhost:8000 -v -H "Origin: https://fake.domain" -X OPTIONS`
 *
 * A failed response can be triggered by changing the domain to any other thing
 *
 * `curl http://localhost:8000 -v -H "Origin: https://notafake.domain" -X OPTIONS`
 */

use misc::SimpleLogger;
mod misc;

async fn index() -> Response {
    Response::ok().body("hello")
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Debug).init().unwrap();
    // We create our tree structure
    let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index));
    // We create a server with the given tree structure
    let server = Server::builder(branch)
        .log_format("[%M %P] %S, from %A")
        .cors(CorsBuilder::new()
            .origin("https://fake.domain")
            .max_age(600)
            .build().unwrap()
        )
        .build().unwrap();
    // And we launch it on the following address
    server.run("127.0.0.1:8000").await.unwrap();
}