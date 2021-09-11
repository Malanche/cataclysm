use tokio::net::{TcpListener, TcpStream};
use crate::{Branch, Shared, additional::Additional, branch::PureBranch, Pipeline, http::{Request, Response}, Error};
use log::{info, error, trace};
use futures::{
    select,
    future::FutureExt,
    channel::oneshot
};
use std::sync::{Arc, Mutex};

/// Builder pattern for the server structure
///
/// It is the main method for building a server and configuring certain behaviour
pub struct ServerBuilder<T> {
    branch: Branch<T>,
    additional: Additional<T>
}

impl<T: Sync + Send> ServerBuilder<T> {
    /// Creates a new server from a given branch
    ///
    /// ```rust,no_run
    /// # use cataclysm::{ServerBuilder, Branch, http::{Method, Response}};
    /// let branch: Branch<()> = Branch::new("/").with(Method::Get.to(|| async {Response::ok().body("Ok!")}));
    /// let mut server_builder = ServerBuilder::new(branch);
    /// // ...
    /// ```
    pub fn new(branch: Branch<T>) -> ServerBuilder<T> {
        ServerBuilder {
            branch,
            additional: Additional {
                shared: None
            }
        }
    }

    /// Declare a information to be shared with the [Shared](crate::Shared) extractor
    ///
    /// ```rust,no_run
    /// use cataclysm::{Server, Branch, Shared, http::{Response, Method, Path}};
    /// 
    /// // Receives a string, and concatenates the shared suffix
    /// async fn index(path: Path<(String,)>, shared: Shared<String>) -> Response {
    ///     let (prefix,) = path.into_inner();
    ///     let suffix = shared.into_inner();
    ///     Response::ok().body(format!("{}{}", prefix, suffix))
    /// }
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     // We create our tree structure
    ///     let branch = Branch::new("/{:prefix}").with(Method::Get.to(index));
    ///     // We create a server with the given tree structure
    ///     let server = Server::builder(branch).share("!!!".into()).build();
    ///     // And we launch it on the following address
    ///     server.run("127.0.0.1:8000").await.unwrap();
    /// }
    /// ```
    ///
    /// If you intend to share a mutable variable, consider using rust's [Mutex](https://doc.rust-lang.org/std/sync/struct.Mutex.html), ad the shared value is already inside an [Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html).
    pub fn share(mut self, shared: T) -> ServerBuilder<T> {
        self.additional.shared = Some(Shared::new(shared));
        self
    }

    /// Builds the server
    ///
    /// ```rust,no_run
    /// use cataclysm::{Server, Branch, Shared, http::{Response, Method, Path}};
    /// 
    /// // Receives a string, and concatenates the shared suffix
    /// async fn index() -> Response {
    ///     Response::ok().body("Hello")
    /// }
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     // We create our tree structure
    ///     let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index));
    ///     // We create a server with the given tree structure
    ///     let server = Server::builder(branch).build();
    ///     // And we launch it on the following address
    ///     server.run("127.0.0.1:8000").await.unwrap();
    /// }
    /// ```
    pub fn build(self) -> Server<T> {
        Server {
            pure_branch: Arc::new(self.branch.purify()),
            additional: Arc::new(self.additional)
        }
    }
}

/// Http Server instance
///
/// The Server structure hosts all the information to successfully process each call
pub struct Server<T> {
    pure_branch: Arc<PureBranch<T>>,
    additional: Arc<Additional<T>>
}

impl<T: 'static + Sync + Send> Server<T> {
    // Short for ServerBuilder's `new` function.
    pub fn builder(branch: Branch<T>) -> ServerBuilder<T> {
        ServerBuilder::new(branch)
    }

    pub async fn run<S: AsRef<str>>(&self, socket: S) -> Result<(), Error> {
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
                        let pure_branch_clone = self.pure_branch.clone();
                        let additional = self.additional.clone();
                        tokio::spawn(async move {
                            match Server::<T>::dispatch(socket, addr, additional, pure_branch_clone).await {
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

    async fn dispatch(socket: TcpStream, addr: std::net::SocketAddr, additional: Arc<Additional<T>>, pure_branch: Arc<PureBranch<T>>) -> Result<(), Error> {
        let request_bytes = Server::<T>::dispatch_read(&socket).await?;

        match Request::parse(request_bytes) {
            Ok(mut request) => {
                request.addr = Some(addr);
                
                // The method will take the request, and modify particularly the "variable count" variable
                let response = match pure_branch.pipeline(&mut request) {
                    Some(pipeline) => {
                        match pipeline {
                            Pipeline::Layer(func, pipeline_layer) => func(request.clone(), pipeline_layer, additional),
                            Pipeline::Core(core_fn) => core_fn(request.clone(), additional)
                        }.await
                    },
                    None => Response::not_found()
                };
                info!("[{} {}] {} from {}", request.method.to_str(), request.path, response.status.0, addr);
                Server::<T>::dispatch_write(socket, response).await?;
            },
            Err(e) => {
                trace!("{}", e);
                Server::<T>::dispatch_write(socket, Response::bad_request()).await?;
            }
        }
        //socket.shutdown();
        Ok(())
    }
}