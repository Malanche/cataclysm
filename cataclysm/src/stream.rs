use tokio::{
    sync::OwnedSemaphorePermit,
    net::TcpStream
};
use bytes::Buf;
use crate::{Error, http::{Response, BasicRequest}};

const CHUNK_SIZE: usize = 4_096;

/// Wrapper around a TCP Stream
pub struct Stream {
    inner: TcpStream,
    permit: Option<OwnedSemaphorePermit>
}

impl Stream {
    /// Generates a new stream
    pub fn new(stream: TcpStream, permit: Option<OwnedSemaphorePermit>) -> Stream {
        Stream{inner: stream, permit}
    }

    pub async fn try_read_response(&self) -> Result<Response, Error> {
        let mut response_bytes = Vec::with_capacity(CHUNK_SIZE);
        // First we read
        loop {
            self.inner.readable().await.map_err(|e| Error::Io(e))?;
            
            // being stored in the async task.
            let mut buf = [0; CHUNK_SIZE];

            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match self.inner.try_read(&mut buf) {
                Ok(0) => {
                    break
                },
                Ok(n) => {
                    response_bytes.extend_from_slice(&buf[0..n]);
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if response_bytes.is_empty() {
                        continue
                    } else {
                        break
                    }
                }
                Err(e) => return Err(Error::Io(e))
            }
        }

        Response::parse(response_bytes)
    }

    /// Writes bytes through the tcp connection
    pub async fn write_bytes<A: AsRef<[u8]>>(&self, bytes: A) -> Result<(), Error> {
        let bytes_ref: &[u8] = bytes.as_ref();
        let mut chunks_iter = bytes_ref.chunks(CHUNK_SIZE);
        #[cfg(feature = "full_log")]
        log::trace!("writting {} chunks of maximum {} bytes each", chunks_iter.len(), CHUNK_SIZE);
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
                        log::debug!("incomplete chunk, trying to serve remaining bytes ({}/{})", current_chunk.len(), CHUNK_SIZE);
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

    /// Allows to send a response through the stream
    pub async fn response(&self, mut response: Response) -> Result<(), Error> {
        self.write_bytes(response.serialize()).await
    }

    /// Allows to send a basic request through the stream
    pub async fn request(&self, basic_request: BasicRequest) -> Result<(), Error> {
        self.write_bytes(basic_request.serialize()).await
    }

    /// Used to retrieve the internal tcp_stream.
    ///
    /// The semaphore permit that might come with it is the helper structure from cataclysm to keep track of the amount of connections that the server has. Use with care.
    pub fn into_tcp_stream(self) -> (TcpStream, Option<OwnedSemaphorePermit>) {
        (self.inner, self.permit)
    }
}

impl std::ops::Deref for Stream {
    type Target = TcpStream;

    // Required method
    fn deref(&self) -> &Self::Target {
        &self.inner
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