use tokio::net::{TcpStream, tcp::OwnedWriteHalf};
use crate::{Error, Frame};
use bytes::Buf;

const CHUNK_SIZE: usize = 4_096;

/// Sending part of web sockets connection
pub struct WebSocketWriter {
    write_stream: OwnedWriteHalf
}

impl WebSocketWriter {
    pub const FIN_RSV: u8 = 0x80;
    pub const OP_CODE_CONTINUATION: u8 = 0x00;
    pub const OP_CODE_TEXT: u8 = 0x01;
    pub const OP_CODE_BINARY: u8 = 0x02;
    pub const OP_CODE_CLOSE: u8 = 0x08;
    pub const OP_CODE_PING: u8 = 0x09;
    pub const OP_CODE_PONG: u8 = 0x0A;

    pub fn new_unchecked(write_stream: OwnedWriteHalf) -> Self {
        WebSocketWriter {
            write_stream
        }
    }

    async fn write<A: Into<Vec<u8>>>(&self, content: A) -> Result<(), Error> {
        let content: Vec<u8> = content.into();
        let mut chunks_iter = content.chunks(CHUNK_SIZE);
        #[cfg(feature = "full_log")]
        log::trace!("writting {} chunks of maximum {} bytes each", chunks_iter.len(), CHUNK_SIZE);
        // We check the first chunk
        let mut current_chunk = match chunks_iter.next() {
            Some(v) => v,
            None => return Ok(()) // Zero length response
        };
        loop {
            // Wait for the socket to be writable
            let stream: &TcpStream = self.write_stream.as_ref();
            stream.writable().await.unwrap();
    
            // Try to write data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match stream.try_write(&current_chunk) {
                Ok(n) => {
                    if n != current_chunk.remaining() {
                        // There are some bytes still to be written in this chunk
                        #[cfg(feature = "full_log")]
                        log::debug!("incomplete chunk, trying to serve remaining bytes ({}/{})", current_chunk.len(), CHUNK_SIZE);
                        current_chunk.advance(n);
                        continue;
                    } else {
                        current_chunk = match chunks_iter.next() {
                            Some(v) => v,
                            None => break Ok(())
                        }
                    }
                }
                Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => break Err(Error::Io(e))
            }
        }
    }

    /// Sends a text message through the websockets connection
    pub async fn text<A: Into<String>>(&self, text: A) -> Result<(), Error> {
        self.write(Frame::text(text)).await
    }

    /// Sends a text message through the websockets connection
    pub async fn bytes<A: Into<Vec<u8>>>(&self, bytes: A) -> Result<(), Error> {
        self.write(Frame::binary(bytes)).await
    }

    /// Sends a ping message through the websockets connection
    pub async fn ping<A: Into<Vec<u8>>>(&self, payload: A) -> Result<(), Error> {
        self.write(Frame::ping(payload)).await
    }

    /// Sends a pong message through the websockets connection
    pub async fn pong<A: Into<Vec<u8>>>(&self, payload: A) -> Result<(), Error> {
        self.write(Frame::pong(payload)).await
    }

    /// Closes the write part of the socket
    pub async fn close(&self) -> Result<(), Error> {
        self.write(Frame::close()).await
    }
}