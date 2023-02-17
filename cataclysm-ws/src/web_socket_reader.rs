use tokio::{
    net::{TcpStream, tcp::OwnedReadHalf},
    task::JoinHandle
};
use crate::{
    Frame,
    Error,
    WebSocketThread,
    communication::read_frame
};

/// Runner thread for a websockets connection
pub struct WebSocketReader {
    read_stream: OwnedReadHalf
}

impl WebSocketReader {
    /// Generates a new instance of the websocket reader, assuming the handshake has already been performed
    pub fn new_unchecked(read_stream: OwnedReadHalf) -> WebSocketReader {
        WebSocketReader {
            read_stream
        }
    }

    /// Blocks until a message is received
    pub async fn try_read_frame(&self) -> Result<Frame, Error> {
        read_frame(&self).await
    }

    /// Spawns a tokio thread that dispatches the message to the proved handler
    pub fn spawn<H: WebSocketThread + 'static>(self, wst: H) -> JoinHandle<<H as WebSocketThread>::Output> {
        WebSocketCustomChild::new(self).spawn(wst)
    }
}

// Reference access to the inner structure
impl AsRef<TcpStream> for WebSocketReader {
    fn as_ref(&self) -> &TcpStream {
        self.read_stream.as_ref()
    }
}

pub struct WebSocketCustomChild {
    automatic_close: bool,
    wsr: WebSocketReader
}

impl WebSocketCustomChild {
    pub fn new(wsr: WebSocketReader) -> WebSocketCustomChild {
        WebSocketCustomChild {
            automatic_close: true,
            wsr
        }
    }

    pub fn automatic_close(mut self, value: bool) -> Self {
        self.automatic_close = value;
        self
    }

    /// Spawns a tokio thread that dispatches the message to the proved handler
    pub fn spawn<H: WebSocketThread + 'static>(self, mut wst: H) -> JoinHandle<<H as WebSocketThread>::Output> {
        tokio::spawn(async move {
            wst.on_open().await;
            loop {
                match self.wsr.try_read_frame().await {
                    Ok(frame) => {
                        if frame.message.is_close() && self.automatic_close {
                            break wst.on_close(true).await
                        }

                        wst.on_message(frame.message).await;
                    },
                    Err(e) => {
                        log::debug!("{}", e);
                        break wst.on_close(false).await
                    }
                }
            }
        })
    }
}

impl From<WebSocketReader> for OwnedReadHalf {
    fn from(source: WebSocketReader) -> OwnedReadHalf {
        source.read_stream
    }
}