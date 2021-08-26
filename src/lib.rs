//! # Cataclysm: A simple http framework
//!
//! Cataclysm is a small personal project, an http framework influenced by [`actix-web`](https://actix.rs/), and built over [`tokio`](https://tokio.rs/). A minimal working example is the following
//! 
//! ```rust,no_run
//! extern crate cataclysm;
//! 
//! use cataclysm::{Server, Path, http::{Response, Method}};
//! 
//! async fn hello() -> Response {
//!     Response::ok().body("hello")
//! }
//! 
//! #[tokio::main]
//! async fn main() {
//!     let server = Server::builder(
//!         Path::new("/hello").with(Method::Get.to(hello))
//!     ).build();
//! 
//!     server.run("localhost:8000").await.unwrap();
//! }
//! ```

use self::error::Error;
mod error;
pub use self::path::Path;
mod path;

/// Contains the specific functionality for http interaction
pub mod http;

pub use self::server::Server;
mod server;

pub use self::metafunctions::{Callback, CoreFn, LayerFn, Pipeline, Extractor};
mod metafunctions;

pub use self::logger::SimpleLogger;
mod logger;

pub use self::session::Session;
mod session;

//use self::api::{ApiResponse};
mod api;
//use tokio::io::AsyncWriteExt;
//use std::io::prelude::*;