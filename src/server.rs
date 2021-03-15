use tokio::net::{TcpListener, TcpStream};
use crate::{Path, Processor, Response, http::{Method, MethodHandler, Request}, Error};
use log::{info, error, trace};
use futures::{
    select,
    future::FutureExt,
    channel::oneshot
};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct ServerBuilder {
    tree: Tree
}

impl ServerBuilder {
    pub fn new(path: Path) -> ServerBuilder {
        let tree = Tree::new(path);
        ServerBuilder {
            tree
        }
    }

    pub fn build(self) -> Server {
        Server {
            tree: Arc::new(RwLock::new(self.tree))
        }
    }
}

/// Structure that holds the entire structure
struct Tree {
    /// Branches that require a simple matching
    branches: HashMap<String, Tree>,
    /// Functions that reply in this specific endpoint
    callees: HashMap<Method, Box<dyn Processor + Send + Sync>>
}

impl Tree {
    fn new(mut path: Path) -> Tree {
        let mut tree = Tree {
            branches: HashMap::new(),
            callees: HashMap::new()
        };

        // Now we finish with the tokens
        if path.tokenized_path.len() == 0 {
            if path.branches.len() != 0 {
                for mut inner_path in path.branches.into_iter() {
                    // We check the composition of the id
                    let id = inner_path.tokenized_path.remove(0);
                    if inner_path.tokenized_path.len() == 0 {
                        let inner_tree = Tree::new(inner_path);
                        tree.branches.insert(id, inner_tree);
                    }
                }
            }
            match path.get_handler.take() {
                Some(handler) => {
                    tree.callees.insert(Method::Get, handler);
                },
                None => ()
            };
        } else {
            // We go deeper
            let id = path.tokenized_path.remove(0);
            let inner_tree = Tree::new(path);
            tree.branches.insert(id, inner_tree);
        }

        tree
    }

    fn get_handler(&self, tokens: Vec<String>) -> Option<&Box<dyn Processor + Send + Sync>> {
        if tokens.len() == 1 {
            if tokens[0] == "" {
                // We go to the get callee
                return self.callees.get(&Method::Get)
            } else {
                return self.branches.get(&tokens[0]).map(|v| v.get_handler(vec!["".to_string()])).flatten();
            }
        } else {
            let mut token_iter = tokens.into_iter();
            let id = token_iter.next().unwrap();
            return self.branches.get(&id).map(|v| v.get_handler(token_iter.collect())).flatten();
        }
    }
}

/// Http Server instance
///
/// The Server structure hosts all the information to successfully process each call
pub struct Server {
    tree: Arc<RwLock<Tree>>
}

impl Server {
    // Short for ServerBuilder's `new` function.
    pub fn builder(path: Path) -> ServerBuilder {
        ServerBuilder::new(path)
    }

    pub async fn run<T: AsRef<str>>(&self, socket: T) -> Result<(), Error> {
        let listener = TcpListener::bind(socket.as_ref()).await.map_err(|e| Error::Io(e))?;
        
        // We use mpsc because ctrlc requires an FnMut function
        let (tx, mut rx) = oneshot::channel::<()>();
        // We put the tx behind an arc mutex
        let tx = Arc::new(Mutex::new(Some(tx)));
        // ctrl + c handler
        ctrlc::set_handler(move || {
            match tx.clone().lock() {
                Ok(mut locked) => match (*locked).take() {
                    Some(tx) => {
                        info!("Shut down requested");
                        tx.send(());
                    },
                    None => {
                        info!("Working on it!");
                    }
                },
                Err(e) => {
                    error!("{}", e);
                }
            }
        }).unwrap();

        loop {
            // We need a fused future for the select macro
            let mut next_connection = Box::pin(listener.accept().fuse());
            
            select! {
                res = next_connection => match res {
                    Ok((socket, _)) => {
                        let tree_clone = self.tree.clone();
                        tokio::spawn(async move {
                            match Server::dispatch(socket, tree_clone).await {
                                Ok(_) => (),
                                Err(e) => {
                                    error!("{}", e);
                                }
                            }
                        });
                    },
                    Err(e) => {
                        error!("{}", e);
                    }
                },
                _ = rx => {
                    info!("Shutting down server");
                    break Ok(())
                }
            };
        }
    }

    /// Deals with the read part of the socket stream
    async fn dispatch_read(socket: &TcpStream) -> Result<Vec<u8>, Error> {
        let mut request_bytes = Vec::with_capacity(4096);
        // First we read
        loop {
            socket.readable().await.map_err(|e| Error::Io(e))?;

            // being stored in the async task.
            let mut buf = [0; 4096];

            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match socket.try_read(&mut buf) {
                Ok(0) => {
                    break
                },
                Ok(n) => request_bytes.extend_from_slice(&buf[0..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(Error::Io(e))
            }
        }
        Ok(request_bytes)
    }

    async fn dispatch_write(socket: TcpStream, mut response: Response) -> Result<(), Error> {
        loop {
            // Wait for the socket to be writable
            socket.writable().await.unwrap();
    
            // Try to write data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match socket.try_write(&response.serialize()) {
            //match socket.try_write(b"Hola mundo\n") {
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

    async fn dispatch(socket: TcpStream, tree: Arc<RwLock<Tree>>) -> Result<(), Error> {
        let request_bytes = Server::dispatch_read(&socket).await?;

        match Request::parse(request_bytes) {
            Ok(request) => {
                let mut token_iter = request.path.split("/");
                // We advance one to skip the whitespace before the root
                let _ = token_iter.next();
                let tokens = token_iter.map(|v| v.to_string()).collect::<Vec<_>>();
                
                let response = match tree.try_read() {
                    Ok(tree) => {
                        match tree.get_handler(tokens) {
                            Some(handler) => {
                                //Response::new().body(b"<div>Hello!!!</div>")
                                handler.handle().await
                            },
                            None => Response::not_found()
                        }
                    },
                    Err(e) => Response::internal_server_error()
                };
                info!("[{} {}] {}", request.method.to_str(), request.path, response.status.0);
                Server::dispatch_write(socket, response).await?;
            },
            Err(e) => {
                trace!("{}", e);
                Server::dispatch_write(socket, Response::bad_request()).await?;
            }
        }
        //socket.shutdown();
        Ok(())
    }
}