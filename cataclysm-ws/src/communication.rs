use tokio::net::{TcpStream};
use crate::{Error, FrameParseError, Message, Frame};

const READ_CHUNK_SIZE: usize = 4_096;

// Helper function to write a message through the websockets write end
pub async fn write_message<A: AsRef<TcpStream>>(stream: A, message: Message) -> Result<(), Error> {
    let content: Vec<u8> = message.into();
    let ref_stream: &TcpStream = stream.as_ref();
    loop {
        // Wait for the socket to be writable
        ref_stream.writable().await.map_err(Error::Io)?;

        // Try to write data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match ref_stream.try_write(&content) {
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

/// Reads a frame from the incoming connection
pub async fn read_frame<A: AsRef<TcpStream>>(stream: A) -> Result<Frame, Error> {
    Frame::parse(read_bytes(stream).await?).map_err(Error::FrameParse)
}

async fn read_bytes<A: AsRef<TcpStream>>(stream: A) -> Result<Vec<u8>, Error> {
    let mut stream_bytes = Vec::with_capacity(READ_CHUNK_SIZE);
    let ref_stream: &TcpStream = stream.as_ref();

    // We need to compute this to receive a full message, no matter the length
    let mut expected_length = None;

    loop {
        // Wait for the socket to be readable
        ref_stream.readable().await.map_err(Error::Io)?;
        let mut buf = [0; READ_CHUNK_SIZE];
        match ref_stream.try_read(&mut buf) {
            Ok(0) => {
                return Err(Error::ConnectionReset);
            }, // will not produce anymore, in theory
            Ok(n) => {
                stream_bytes.extend_from_slice(&buf[0..n]);
                
                if let Some(expected_length) = &expected_length {
                    if stream_bytes.len() != *expected_length {
                        continue
                    } else {
                        break;
                    }
                } else {
                    match Frame::parse(&stream_bytes) {
                        Ok(_) => break,
                        Err(FrameParseError::Incomplete{expected, ..}) => {
                            expected_length = Some(expected);
                            continue
                        },
                        Err(e) => {
                            return Err(Error::FrameParse(e))
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue
            }
            Err(e) => {
                return Err(Error::Io(e));
            }
        }
    }

    Ok(stream_bytes)
}