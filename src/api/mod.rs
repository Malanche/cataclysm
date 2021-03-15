extern crate serde;

use crate::http::Response;
use serde::de::{DeserializeOwned};

pub trait ApiResponse: DeserializeOwned {
    fn handle(&self) -> Response;
}