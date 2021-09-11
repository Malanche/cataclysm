# Cataclysm. A simple rust http server

**Work in progress**: The work is completely unusable in its current state, but updates will come soon until a full HTTP/1.1 server is functional. This legend will be removed when such a state is reached.

Cataclysm is an http framework influenced by [actix-web](https://actix.rs/), and built over [tokio](https://tokio.rs/). A minimal working example is the following

```rust
use cataclysm::{Server, Branch, http::{Response, Method}};

async fn hello() -> Response {
    Response::ok().body("hello")
}

#[tokio::main]
async fn main() {
    let server = Server::builder(
        Branch::<()>::new("/hello").with(Method::Get.to(hello))
    ).build();

    server.run("localhost:8000").await.unwrap();
}
```

## Closures as callbacks

Until `async closures` become stable, the option to pass closures as a path handler is with a closure that returns an async block

```rust
use cataclysm::{Server, Branch, http::{Response, Method}};

#[tokio::main]
async fn main() {
    let server = Server::builder(
        Branch::<()>::new("/data").with(Method::Post.to(|| async {
            Response::ok().body("worked!")
        }))
    ).build();

    server.run("localhost:8000").await.unwrap();
}
```

## Extractors

Some data can be retrieved from an http request by just adding arguments to the callback, with types that implement the `Extractor` trait. The default implementation list is the following

* `String`: Tries to extract the body as a valid utf-8 string. Returns Bad-Request if the operation fails
* `Vec<u8>`: Returns the content of the `http` call as a stream of bytes
* `Request`: Returns the request for a bit more control within the callback
* `Path<T>`: Returns the parameters from the path. T must be a tuple.
* `Shared<T>`: Returns the shared data provided to the server (if any).

## Sharing data to the functions from the server

Data can be shared accross the server calls through the `share` method from the `ServerBuilder` structure, and with the help of the `Shared` structure.

```rust
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
```

If you want to share mutable data, then use rust's `Mutex` structure (as the `Shared` structure already provides an `Arc` wrapper).

## Layers

Cataclysm allows for layer handling, a.k.a. middleware.

```rust
use cataclysm::{Server, Branch, Additional, Pipeline, http::{Response, Request, Method}};
use std::sync::Arc;
use futures::future::FutureExt;

#[tokio::main]
async fn main() {
    let branch = Branch::new("/").with(Method::Get.to(|| async {Response::ok()}))
        .layer(|req: Request, pipeline: Box<Pipeline<()>>, ad: Arc<Additional<()>>| async {
            // Example of timing layer
            println!("Time measuring begins");
            let now = std::time::Instant::now();
            let request = pipeline.execute(req, ad).await;
            let elapsed = now.elapsed().as_nanos();
            println!("Process time: {} ns", elapsed);
            request
        }.boxed());
    let server = Server::builder(branch).build();
    server.run("localhost:8000").await.unwrap();
}
```

As seen in the example, layer functions receive a `Request` and a boxed `Pipeline` enum. The `Pipeline` enum contains a nested structure of futures (the layers + the core handler), and has the `execute` to simplify things a bit. This function must return a `Pin<Box<_>>` future, so either use the `boxed` method from the `FutureExt` trait from the `futures` crate, or wrap it manually.

### TODO

- [ ] Regex with / cause problems in branch creation (fix with queue implementation for "{", "}" detection)