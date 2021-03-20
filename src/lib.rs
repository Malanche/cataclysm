//! # Cataclysm: A simple http framework
//!
//! Cataclysm is a small personal proyect that uses [`tokio`](https://docs.rs/tokio) to build a small, simple and fast asynchronous web server.
//!
//! An example of minimal code to start a server is the following
//! 
//! ```
//! use cataclysm::{Server, Path, http::{Response, Method}};
//! 
//! async fn hello_world() -> Response {
//!     Response::ok().body("hola mundo")
//! }
//! 
//! #[tokio::main]
//! async fn main() {
//!     let server = Server::builder(
//!         Path::new("/").with(Method::Get.to(hello_world))
//!     ).build();
//! 
//!     server.run("localhost:8000").await.unwrap();
//! }
//! ```

use self::error::Error;
mod error;
pub use self::path::Path;
mod path;

pub mod http;

pub use self::server::Server;
mod server;

pub use self::metafunctions::{Callback, Extractor};
mod metafunctions;

pub use self::logger::SimpleLogger;
mod logger;

//use self::api::{ApiResponse};
mod api;
//use tokio::io::AsyncWriteExt;
//use std::io::prelude::*;