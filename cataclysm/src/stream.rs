use tokio::net::TcpStream;
use bytes::Buf;
use crate::{Error, http::Response};

const RESPONSE_CHUNK_SIZE: usize = 4_096;

pub struct Stream {
    inner: TcpStream
}

impl Stream {
    pub fn new(stream: TcpStream) -> Stream {
        Stream{inner: stream}
    }

    pub async fn reply(&self, mut response: Response) -> Result<(), Error> {
        let serialized_response = response.serialize();
        let mut chunks_iter = serialized_response.chunks(RESPONSE_CHUNK_SIZE);
        #[cfg(feature = "full_log")]
        log::trace!("writting {} chunks of maximum {} bytes each", chunks_iter.len(), RESPONSE_CHUNK_SIZE);
        // We check the first chunk
        let mut current_chunk = match chunks_iter.next() {
            Some(v) => v,
            None => return Ok(()) // Zero length response
        };
        loop {
            // Wait for the socket to be writable
            self.inner.writable().await.map_err(|e| Error::Io(e))?;
    
            // Try to write data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.        
            match self.inner.try_write(&current_chunk) {
                Ok(n) => {
                    if n != current_chunk.remaining() {
                        // There are some bytes still to be written in this chunk
                        #[cfg(feature = "full_log")]
                        log::debug!("incomplete chunk, trying to serve remaining bytes ({}/{})", current_chunk.len(), RESPONSE_CHUNK_SIZE);
                        current_chunk.advance(n);
                        continue;
                    } else {
                        current_chunk = match chunks_iter.next() {
                            Some(v) => v,
                            None => return Ok(())
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
}

// Reference access to the inner structure
impl AsRef<TcpStream> for Stream {
    fn as_ref(&self) -> &TcpStream {
        &self.inner
    }
}

// Mutable reference access to the inner structure
impl AsMut<TcpStream> for Stream {
    fn as_mut(&mut self) -> &mut TcpStream {
        &mut self.inner
    }
}

// Conversion to inner type
impl Into<TcpStream> for Stream {
    fn into(self) -> TcpStream {
        self.inner
    }
}