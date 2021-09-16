//! # Cataclysm: A simple http framework
//!
//! Cataclysm is a small personal project, an http framework influenced by [`actix-web`](https://actix.rs/), and built over [`tokio`](https://tokio.rs/). A minimal working example is the following
//! 
//! ```rust,no_run
//! extern crate cataclysm;
//! 
//! use cataclysm::{Server, Branch, http::{Response, Method}};
//! 
//! async fn index() -> Response {
//!     Response::ok().body("Hello, World!")
//! }
//! 
//! #[tokio::main]
//! async fn main() {
//!     let server = Server::builder(
//!         Branch::<()>::new("/").with(Method::Get.to(index))
//!     ).build().unwrap();
//! 
//!     server.run("localhost:8000").await.unwrap();
//! }
//! ```

pub use self::error::Error;
mod error;
pub use self::branch::{Branch};
mod branch;

/// Contains the specific functionality for http interaction
pub mod http;

pub use self::server::{Server, ServerBuilder};
mod server;
pub use self::shared::{Shared};
mod shared;
pub use self::additional::Additional;
mod additional;

pub use self::metafunctions::{Callback, CoreFn, LayerFn, WebsocketFn, Pipeline, Extractor};
mod metafunctions;

pub use self::session::Session;
mod session;

/// Contains some basic websockets functionality
pub mod ws;