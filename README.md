# Cataclysm. A simple rust http server

**Work in progress.**

Example of minimal code

```Rust
use cataclysm::{Server, Path, http::{Response, Method}, SimpleLogger};

async fn hello_world() -> Response {
    Response::ok().body("hola mundo")
}

#[tokio::main]
async fn main() {
    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello_world))
    ).build();

    server.run().await.unwrap();
}
```

#### Progress

The work is completely unusable in its current state, but updates will come soon until a full HTTP/1.1 server is functional.