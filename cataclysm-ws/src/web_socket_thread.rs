use crate::Message;

/// Trait necessary to start a ws read-processing thread
#[async_trait::async_trait]
pub trait WebSocketThread: Send {
    type Output: Send;
    /// On opened connection
    ///
    /// This function gets called when the websockets connection is properly stablished.
    async fn on_open(&mut self) {}
    /// On message callback
    ///
    /// This function gets called back when a [Message](crate::Message) is received.
    async fn on_message(&mut self, message: Message);
    
    /// On closed connection
    ///
    /// This function gets called when the websockets connection is closed (either gracefully, or by an error)
    async fn on_close(&mut self, _clean: bool) -> Self::Output;
}