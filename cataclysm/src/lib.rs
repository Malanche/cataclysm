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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

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
pub use self::cors::{CorsBuilder, Cors};
mod cors;

pub use self::metafunctions::{Callback, CoreFn, LayerFn, Pipeline, Extractor};
#[cfg(feature = "stream")]
pub use self::metafunctions::{StreamCallback};
#[cfg(feature = "stream")]
pub(crate) use self::metafunctions::{HandlerFn};
mod metafunctions;

/// Contains usefull stuff for session management
pub mod session;

pub use self::stream::Stream;
mod stream;

/// Contains some basic websockets functionality
#[cfg(feature = "ws")]
pub mod ws;