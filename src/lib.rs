use self::error::Error;
mod error;
pub use self::path::Path;
mod path;

use self::http::{Response, MethodHandler};
pub mod http;

use self::api::{ApiResponse};
mod api;

extern crate tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use std::io::prelude::*;

pub struct Server {
    paths: Vec<(String, Path)>
}

impl Server {
    pub fn new() -> Server {
        Server {paths: Vec::new()}
    }

    pub async fn run(&self) -> Result<(), Error> {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

        loop {
            let (mut socket, _) = listener.accept().await.unwrap();

            tokio::spawn(Server::socket_handler(socket));
        }
        Ok(())
    }

    async fn socket_handler(mut socket: TcpStream) -> Result<(), Error> {
        // We wait for the stream to be writable...
        
        loop {
            // Wait for the socket to be writable
            socket.writable().await.unwrap();
    
            // Try to write data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match socket.try_write(&Response::new().body(b"<div>Hello!!!</div>").serialize()) {
            //match socket.try_write(b"Hola mundo\n") {
                Ok(n) => {
                    break;
                }
                Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                    println!("Writing second");
                    continue;
                }
                Err(e) => panic!("{}", e)
            }
        }
        //socket.shutdown();
        Ok(())
    }

    pub fn path(mut self, path: Path) -> Self {
        self.paths.push((path.get_root(), path));
        self
    }
}