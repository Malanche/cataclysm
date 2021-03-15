extern crate ctrlc;

use self::error::Error;
mod error;
pub use self::path::Path;
mod path;

use self::http::{Response, Request};
pub mod http;

pub use self::server::Server;
mod server;

pub use self::processor::Processor;
mod processor;

pub use self::logger::SimpleLogger;
mod logger;

//use self::api::{ApiResponse};
mod api;
//use tokio::io::AsyncWriteExt;
//use std::io::prelude::*;