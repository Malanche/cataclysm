pub use cataclysm_ws::{Error, WebSocketReader, WebSocketWriter, WebSocketThread, Message, Frame};
use std::sync::Arc;

/// Factory trait for the `websocket_factory` and `demon_factory` methods
#[async_trait::async_trait]
pub trait WebSocketFactory<T> {
    /// Function that receives a [WebSocketWriter](crate::ws::WebSocketWriter), and returns the type `T` (which normally implements [WebSocketReader](crate::ws::WebSocketReader) or both [WebSocketReader](crate::ws::WebSocketReader) and `Demon`).
    async fn create(self: Arc<Self>, web_socket_writer: WebSocketWriter) -> T;
}