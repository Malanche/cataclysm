pub use self::web_socket_reader::WebSocketReader;
pub use self::web_socket_writer::WebSocketWriter;
pub use self::frame::Frame;
pub use self::message::Message;
mod web_socket_reader;
mod web_socket_writer;
mod frame;
mod message;

use crate::Error;
use tokio::{
    io::AsyncReadExt,
    net::{tcp::OwnedReadHalf}
};
use bytes::{BytesMut};

pub(crate) struct WebSocketThread {
    read_stream: OwnedReadHalf,
    web_socket_reader: Box<dyn WebSocketReader>
}

impl WebSocketThread {
    pub fn spawn(read_stream: OwnedReadHalf, web_socket_reader: Box<dyn WebSocketReader>) {
        let mut web_socket_thread = WebSocketThread {
            read_stream,
            web_socket_reader
        };

        tokio::spawn(async move {
            web_socket_thread.web_socket_reader.on_open().await;
            match web_socket_thread.read_loop().await {
                Ok(_) => log::debug!("Leaving read loop in a nice manner"),
                Err(e) => log::debug!("Leaving read loop with error, {}", e)
            };
            web_socket_thread.web_socket_reader.on_close().await;
        });
    }

    async fn read_loop(&mut self) -> Result<(), Error> {
        // Outter loop for processing messages
        let mut buf = BytesMut::with_capacity(8 * 1024);
        loop {
            // Inner loop for reading each message
            let maybe_frame = loop {
                match Frame::parse(&buf) {
                    Ok(frame) => break Ok(Some(frame)),
                    Err(Error::Parse(e)) => {
                        log::debug!("{}, clearing buffer", e);
                        buf.clear();
                    },
                    Err(Error::Incomplete) => (),
                    Err(e) => return Err(e)
                };

                if 0 == self.read_stream.read_buf(&mut buf).await.unwrap() {
                    // Closed connection!
                    if buf.is_empty() {
                        break Ok(None);
                    } else {
                        break Err(Error::ConnectionReset);
                    }
                }
            }?;

            if let Some(frame) = maybe_frame {
                // We got a correct message, we clear the buffer
                buf.clear();
                // And call the handler
                if let Some(message) = frame.message {
                    self.web_socket_reader.on_message(message).await;
                }
            } else {
                // Closing the connection in a nice way
                break Ok(());
            }
        }
    }
}