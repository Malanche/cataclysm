use tokio::net::{TcpStream, tcp::OwnedWriteHalf};
use crate::{Error, ws::Frame};

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

    pub(crate) fn new(write_stream: OwnedWriteHalf) -> Self {
        WebSocketWriter {
            write_stream
        }
    }

    pub async fn text<A: Into<String>>(&self, text: A) -> Result<(), Error> {
        let content = Frame::text(text).bytes();
        loop {
            // Wait for the socket to be writable
            let stream: &TcpStream = self.write_stream.as_ref();
            stream.writable().await.unwrap();
    
            // Try to write data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match stream.try_write(&content) {
                Ok(_n) => {
                    break Ok(());
                }
                Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => break Err(Error::Io(e))
            }
        }
    }
}