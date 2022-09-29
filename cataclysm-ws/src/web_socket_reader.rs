use crate::Message;

/// Receiving part of web sockets connection
///
/// By default, a threaded runner is used to keep the websocket connections alive.
#[async_trait::async_trait]
pub trait WebSocketReader: Send {
    /// On opened connection
    ///
    /// This function gets called when the websockets connection is properly stablished.
    async fn on_open(&mut self) {
        ()
    }
    /// On message callback
    ///
    /// This function gets called back when a [Message](crate::Message) is received.
    async fn on_message(&mut self, message: Message);
    /// On opened connection
    ///
    /// This function gets called when the websockets connection is closed (either gracefully, or by an error)
    async fn on_close(&mut self) {
        ()
    }
}