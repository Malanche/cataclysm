use tokio::{
    net::{TcpStream},
    sync::{OwnedSemaphorePermit}
};
use crate::{Error, Message, Frame, WebSocketWriter, WebSocketReader};
use crate::communication::{write_message, read_frame};

/// Wrapper structure of a tcp stream with some websockets utilities
pub struct WebSocketStream {
    inner: TcpStream,
    permit: Option<OwnedSemaphorePermit>
}

impl WebSocketStream {
    /// Wraps a tcp stream withot checking the handshake or anything
    pub fn from_tcp_stream_unchecked(stream: TcpStream) -> WebSocketStream {
        WebSocketStream {
            inner: stream,
            permit: None
        }
    }

    /// Auxiliar function that cataclysm uses to keep track of connections
    pub fn set_permit(&mut self, permit: OwnedSemaphorePermit) {
        self.permit = Some(permit);
    }

    /// Sends a message through the websockets connection
    pub async fn send_message(&self, message: Message) -> Result<(), Error> {
        write_message(&self, message).await
    }

    /// Blocks until a message is received
    pub async fn try_read_frame(&self) -> Result<Frame, Error> {
        read_frame(&self).await
    }

    /// Splits the stream into the reading and writting part
    pub fn split(self) -> (WebSocketWriter, WebSocketReader) {
        let (rx, tx) = self.inner.into_split();
        let mut web_socket_reader = WebSocketReader::new_unchecked(rx);
        if let Some(permit) = self.permit {
            web_socket_reader.set_permit(permit);
        }
        (WebSocketWriter::new_unchecked(tx), web_socket_reader)
    }
}

// Reference access to the inner structure
impl AsRef<TcpStream> for WebSocketStream {
    fn as_ref(&self) -> &TcpStream {
        &self.inner
    }
}

// Mutable reference access to the inner structure
impl AsMut<TcpStream> for WebSocketStream {
    fn as_mut(&mut self) -> &mut TcpStream {
        &mut self.inner
    }
}

// Conversion to inner type
impl Into<TcpStream> for WebSocketStream {
    fn into(self) -> TcpStream {
        self.inner
    }
}