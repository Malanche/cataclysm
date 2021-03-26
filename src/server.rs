use tokio::net::{TcpListener, TcpStream};
use crate::{Path, CoreFn, LayerFn, Pipeline, http::{Method, Request, Response}, Error};
use log::{info, error, trace};
use futures::{
    select,
    future::FutureExt,
    channel::oneshot
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct ServerBuilder {
    tree: Tree
}

/// Builder pattern for the server structure
///
/// 
impl ServerBuilder {
    pub fn new(path: Path) -> ServerBuilder {
        let tree = Tree::new(path);
        ServerBuilder {
            tree
        }
    }

    pub fn build(self) -> Server {
        Server {
            tree: Arc::new(self.tree)
        }
    }
}

/// Structure that holds the entire structure
struct Tree {
    /// Branches that require a simple matching
    branches: HashMap<String, Tree>,
    /// Functions that reply in this specific endpoint
    callees: HashMap<Method, CoreFn>,
    /// Default function to be called if necessary
    default_callee: Option<CoreFn>,
    /// Middleware functions
    layer_functions: Vec<Arc<LayerFn>>
}

impl Tree {
    fn new(mut path: Path) -> Tree {
        let mut tree = Tree {
            branches: HashMap::new(),
            callees: HashMap::new(),
            default_callee: None,
            layer_functions: path.layer_functions.drain(..).collect()
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
            // We add the method calls in this level of the tree
            for (method, handler) in path.method_handlers.into_iter() {
                tree.callees.insert(method, handler);
            }
            // We copy the default callee
            tree.default_callee = path.default_method;
        } else {
            // We go deeper
            let id = path.tokenized_path.remove(0);
            let inner_tree = Tree::new(path);
            tree.branches.insert(id, inner_tree);
        }
        tree
    }

    fn get_handler(&self, tokens: Vec<String>, method: &Method) -> Option<&CoreFn> {
        if tokens.len() == 1 {
            // This means we are in the end of the tree
            if tokens[0] == "" {
                // We go to the get callee
                self.callees.get(&method).or(self.default_callee.as_ref())
            } else {
                // We are in the last branch
                return self.branches.get(&tokens[0]).map(|v| {
                    v.get_handler(vec!["".to_string()], method)
                }).flatten().or(self.default_callee.as_ref());
            }
        } else {
            // We need to keep walking the tree
            let mut token_iter = tokens.into_iter();
            let id = token_iter.next().unwrap();
            return self.branches.get(&id).map(|v| {
                v.get_handler(token_iter.collect(), method)
            }).flatten().or(self.default_callee.as_ref());
        }
    }

    fn get_pipeline(&self, tokens: Vec<String>, method: &Method) -> (Option<&CoreFn>, Vec<Arc<Layer>>) {
        if tokens.len() == 1 {
            // This means we are in the end of the tree
            if tokens[0] == "" {
                // We go to the get callee
                self.callees.get(&method).or(self.default_callee.as_ref())
            } else {
                // We are in the last branch
                return self.branches.get(&tokens[0]).map(|v| {
                    v.get_handler(vec!["".to_string()], method)
                }).flatten().or(self.default_callee.as_ref());
            }
        } else {
            // We need to keep walking the tree
            let mut token_iter = tokens.into_iter();
            let id = token_iter.next().unwrap();
            return self.branches.get(&id).map(|v| {
                v.get_handler(token_iter.collect(), method)
            }).flatten().or(self.default_callee.as_ref());
        }
    }
}

/// Http Server instance
///
/// The Server structure hosts all the information to successfully process each call
pub struct Server {
    tree: Arc<Tree>
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
                        match tx.send(()) {
                            Ok(_) => (),
                            Err(_) => error!("could not complete request")
                        };
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
                    Ok((socket, addr)) => {
                        let tree_clone = self.tree.clone();
                        tokio::spawn(async move {
                            match Server::dispatch(socket, addr, tree_clone).await {
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

    async fn dispatch(socket: TcpStream, addr: std::net::SocketAddr, tree: Arc<Tree>) -> Result<(), Error> {
        let request_bytes = Server::dispatch_read(&socket).await?;

        match Request::parse(request_bytes) {
            Ok(mut request) => {
                request.addr = Some(addr);
                let mut token_iter = request.path.split("/");
                // We advance one to skip the whitespace before the root
                let _ = token_iter.next();
                let tokens = token_iter.map(|v| v.to_string()).collect::<Vec<_>>();
                let response = match tree.get_handler(tokens, &request.method) {
                    Some(handler) => {
                        /*
                        type Middleware = Box<dyn Fn(&Request, std::vec::IntoIter<Middleware>) -> std::pin::Pin<Box<dyn futures::Future<Output = Response> + Send>> + Send + Sync>;
                        let middlewares = vec![Box::new(|req: &Request, layers: std::vec::IntoIter<Middleware>| async {
                            layers.next().unwrap()(req, layers).await
                        })];
                        */
                        //middleware(&request)
                        handler(request.clone()).await
                    },
                    None => Response::not_found()
                };
                info!("[{} {}] {} from {}", request.method.to_str(), request.path, response.status.0, addr);
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