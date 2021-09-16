use crate::ws::Message;

/// Receiving part of web sockets connection
///
/// By default, a threaded runner is used to keep the websocket connections alive. In case you would like to use `apocalypse` as a runner, you have to enable such feature.
#[async_trait::async_trait]
pub trait WebSocketReader: Send {
    /// On message callback
    async fn on_message(&mut self, message: Message);
}