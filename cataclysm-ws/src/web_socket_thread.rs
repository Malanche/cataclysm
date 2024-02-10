use std::future::Future;
use crate::Message;

/// Trait necessary to start a ws read-processing thread
pub trait WebSocketThread: Send + 'static {
    type Output: Send;
    /// On opened connection
    ///
    /// This function gets called when the websockets connection is properly stablished.
    fn on_open(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }
    /// On message callback
    ///
    /// This function gets called back when a [Message](crate::Message) is received.
    fn on_message(&mut self, message: Message) -> impl Future<Output = ()> + Send;
    
    /// On closed connection
    ///
    /// This function gets called when the websockets connection is closed (either gracefully, or by an error)
    fn on_close(&mut self, _clean: bool) -> impl Future<Output = Self::Output> + Send;
}