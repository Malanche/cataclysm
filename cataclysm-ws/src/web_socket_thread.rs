use tokio::{
    io::AsyncReadExt,
    net::{tcp::OwnedReadHalf}
};
use bytes::{BytesMut};
use crate::{WebSocketReader, Error, Frame};

/// Runner thread for a websockets connection
pub struct WebSocketThread {
    read_stream: OwnedReadHalf
}

impl WebSocketThread {
    pub fn new(read_stream: OwnedReadHalf) -> WebSocketThread {
        WebSocketThread {
            read_stream
        }
    }
    /// Spawns the websocket thread that handles the 
    pub fn spawn<A: 'static + WebSocketReader>(mut self, mut web_socket_reader: A) {
        tokio::spawn(async move {
            web_socket_reader.on_open().await;
            match self.read_loop(web_socket_reader).await {
                Ok(_) => log::debug!("Leaving read loop in a nice manner"),
                Err(e) => log::debug!("Leaving read loop with error, {}", e)
            };
        });
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Error> {
        let mut buf = BytesMut::with_capacity(8 * 1024);
        loop {
            match Frame::parse(&buf) {
                Ok(frame) => break Ok(Some(frame)),
                Err(Error::Parse(e)) => {
                    // Corrupt message, we clear the buffer and keep waiting for messages
                    log::debug!("{}, clearing buffer", e);
                    buf.clear();
                },
                Err(Error::Incomplete) => (),
                Err(e) => return Err(e)
            };

            if 0 == self.read_stream.read_buf(&mut buf).await.map_err(Error::Io)? {
                // Closed connection!
                if buf.is_empty() {
                    break Ok(None);
                } else {
                    break Err(Error::ConnectionReset);
                }
            }
        }
    }

    async fn read_loop<A: WebSocketReader>(&mut self, mut web_socket_reader: A) -> Result<(), Error> {
        // Outter loop for processing messages
        let mut buf = BytesMut::with_capacity(8 * 1024);
        loop {
            // Inner loop for reading each message
            let maybe_frame = self.read_frame().await?;

            if let Some(frame) = maybe_frame {
                // We got a correct message, we clear the buffer
                buf.clear();
                // And call the handler
                if let Some(message) = frame.message {
                    web_socket_reader.on_message(message).await;
                } else if frame.is_close() {
                    web_socket_reader.on_close(true).await;
                    break Ok(())
                }
            } else {
                // Closing the connection in a nice way
                web_socket_reader.on_close(false).await;
                break Ok(());
            }
        }
    }
}

impl From<WebSocketThread> for OwnedReadHalf {
    fn from(source: WebSocketThread) -> OwnedReadHalf {
        source.read_stream
    }
}