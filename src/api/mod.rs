use crate::http::Response;

pub trait ApiResponse {
    fn handle() -> Response;
}