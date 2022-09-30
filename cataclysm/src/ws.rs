pub use cataclysm_ws::{Error, WebSocketReader, WebSocketWriter, WebSocketThread, Message, Frame};
use std::sync::Arc;

/// Factory trait for the `websocket_factory` and `demon_factory` methods
#[async_trait::async_trait]
pub trait WebSocketHandler {
    /// Function that receives a [WebSocketWriter](crate::ws::WebSocketWriter) and a [WebSocketThread](crate::ws::WebSocketThread)
    async fn create(self: Arc<Self>, wst: WebSocketThread, wsw: WebSocketWriter);
}