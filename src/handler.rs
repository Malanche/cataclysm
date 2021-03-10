/// Something like an alias for a handler function
pub trait Handler {
    fn handle(request: Request) -> Response
}