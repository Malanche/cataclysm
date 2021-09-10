# Cataclysm. A simple rust http server

**Work in progress**: The work is completely unusable in its current state, but updates will come soon until a full HTTP/1.1 server is functional. This legend will be removed when such a state is reached.

Cataclysm is an http framework influenced by [actix-web](https://actix.rs/), and built over [tokio](https://tokio.rs/). A minimal working example is the following

```Rust
extern crate cataclysm;

use cataclysm::{Server, Branch, http::{Response, Method}};

async fn hello() -> Response {
    Response::ok().body("hello")
}

#[tokio::main]
async fn main() {
    let server = Server::builder(
        Branch::new("/hello").with(Method::Get.to(hello))
    ).build();

    server.run("localhost:8000").await.unwrap();
}
```

## Closures as callbacks

Until `async closures` become stable, the option to pass closures as a path handler is with a closure that returns an async block

```Rust
let server = Server::builder(
    Branch::new("/data").with(Method::Post.to(|data: Vec<u8>| async {
        // Do something with data
        // ...
        Response::ok().body("worked!")
    }))
).build();
```

## Extractors

Some data can be retrieved from an http request by just adding arguments to the callback, with types that implement the `Extractor` trait. The default implementation list is the following

* `String`: Tries to extract the body as a valid utf-8 string. Returns Bad-Request if the operation fails
* `Vec<u8>`: Returns the content of the `http` call as a stream of bytes
* `Request`: Returns the request for a bit more control within the callback
* `Path<T>`: Returns the parameters from the path. T must be a tuple.

## Layers

Cataclysm allows for layer handling, a.k.a. middleware.

```Rust
let path = Branch::new("/hello")
    .with(Method::Get.to(world))
    .layer(|req: Request, pipeline: Box<Pipeline>| async {
        // Example of timing layer / middleware
        let now = std::time::Instant::now();
        // Execute the deeper layers of the pipeline
        let response = pipeline.execute(req).await;
        // Measure and print time
        let elapsed = now.elapsed().as_nanos();
        info!("Process time: {} ns", elapsed);
        // We return the request for further possible processing.
        request
}.boxed()).nested(Branch::new("/hola").with(Method::Delete.to(hello)));
```

As seen in the example, layer functions receive a `Request` and a boxed `Pipeline` enum. The `Pipeline` enum contains a nested structure of futures (the layers + the core handler), and has the `execute` to simplify things a bit. This function must return a `Pin<Box<_>>` future, so either use the `boxed` method from the `FutureExt` trait from the `futures` crate, or wrap it manually.

### TODO

- [ ] Regex with / cause problems in branch creation (fix with queue implementation for "{", "}" detection)