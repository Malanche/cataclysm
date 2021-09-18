use tokio::{
    sync::Semaphore,
    net::{TcpListener, TcpStream}
};
use crate::{Branch, Shared, Additional, branch::PureBranch, Pipeline, http::{Request, Response}, Error};
#[cfg(feature = "ws")]
use crate::ws::{WebSocketThread, WebSocketWriter};
#[cfg(feature = "demon")]
use apocalypse::Gate;
use log::{info, error};
use std::sync::{Arc};
use ring::{hmac::{self, Key}, rand};

// Default max connections for the server
const MAX_CONNECTIONS: usize = 2_000;

/// Builder pattern for the server structure
///
/// It is the main method for building a server and configuring certain behaviour
pub struct ServerBuilder<T> {
    branch: Branch<T>,
    shared: Option<Shared<T>>,
    secret: Option<Key>,
    log_string: Option<String>,
    max_connections: usize,
    timeout: std::time::Duration,
    #[cfg(feature = "demon")]
    gate: Option<Gate>
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
            shared: None,
            secret: None,
            log_string: None,
            max_connections: MAX_CONNECTIONS,
            timeout: std::time::Duration::from_millis(15_000),
            #[cfg(feature = "demon")]
            gate: None
        }
    }

    /// Declare some information to be shared with the [Shared](crate::Shared) extractor
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
    ///     let server = Server::builder(branch).share("!!!".into()).build().unwrap();
    ///     // And we launch it on the following address
    ///     server.run("127.0.0.1:8000").await.unwrap();
    /// }
    /// ```
    ///
    /// If you intend to share a mutable variable, consider using rust's [Mutex](https://doc.rust-lang.org/std/sync/struct.Mutex.html), ad the shared value is already inside an [Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html).
    pub fn share(mut self, shared: T) -> ServerBuilder<T> {
        self.shared = Some(Shared::new(shared));
        self
    }

    /// Sets a custom `Key` for cookie signature
    ///
    /// ```rust,no_run
    /// use cataclysm::{Server, Session, Branch, Shared, http::{Response, Method, Path}};
    /// 
    /// async fn index(session: Session) -> Response {
    ///     // the session will be empty if the signature was invalid
    ///     // ... do something with the session
    ///     // apply changes to response
    ///     session.apply(Response::ok())
    /// }
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     // We create our tree structure
    ///     let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index));
    ///     // We create a server with the given tree structure
    ///     let server = Server::builder(branch).secret("very secret").build().unwrap();
    ///     // And we launch it on the following address
    ///     server.run("127.0.0.1:8000").await.unwrap();
    /// }
    /// ```
    ///
    /// If no secret is provided, a random key will be used (generated by ring).
    pub fn secret<A: AsRef<[u8]>>(mut self, secret: A) -> Self {
        self.secret = Some(hmac::Key::new(hmac::HMAC_SHA256, secret.as_ref()));
        self
    }

    /// Sets a log string, to log information per call
    ///
    /// ```rust,no_run
    /// # use cataclysm::{Server, Branch, Shared, http::{Response, Method, Path}};
    /// // Tree structure
    /// let branch: Branch<()> = Branch::new("/").with(Method::Get.to(|| async {Response::ok()}));
    /// // Now we configure the server
    /// let server = Server::builder(branch).log_format("[%M %P] %S, from %A").build().unwrap();
    /// ```
    ///
    /// The list of available format elements are the following
    /// 
    /// * `%M`: Method from the request
    /// * `%P`: Path from the request
    /// * `%S`: Status from the response
    /// * `%A`: Socket address and port from the connection
    /// (more data to be added soon)
    pub fn log_format<A: Into<String>>(mut self, log_string: A) -> Self {
        self.log_string = Some(log_string.into());
        self
    }

    /// Sets up a maximum number of connections for the server to be dealt with
    ///
    /// ```rust,no_run
    /// # use cataclysm::{Server, Branch, http::{Response, Method}};
    /// // Tree structure
    /// let branch: Branch<()> = Branch::new("/").with(Method::Get.to(|| async {Response::ok()}));
    /// // Now we configure the server
    /// let server = Server::builder(branch).max_connections(10_000).build().unwrap();
    /// ```
    pub fn max_connections(mut self, n: usize) -> Self {
        self.max_connections = n;
        self
    }

    /// Sets up a custom timeout for http requests to be finished
    ///
    /// ```rust,no_run
    /// # use cataclysm::{Server, Branch, http::{Response, Method}};
    /// use std::time::Duration;
    /// // Tree structure
    /// let branch: Branch<()> = Branch::new("/").with(Method::Get.to(|| async {Response::ok()}));
    /// // Now we configure the server
    /// let server = Server::builder(branch).timeout(Duration::from_millis(5_000)).build().unwrap();
    /// ```
    pub fn timeout(mut self, duration: std::time::Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Sets up a gate for demon spawning
    ///
    /// See the [demon_factory](crate::Branch::demon_factory) documentation from the [Branch](crate::Branch) structure for more details about this function.
    #[cfg(feature = "demon")]
    pub fn gate(mut self, gate: Gate) -> Self {
        self.gate = Some(gate);
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
    ///     let server = Server::builder(branch).build().unwrap();
    ///     // And we launch it on the following address
    ///     server.run("127.0.0.1:8000").await.unwrap();
    /// }
    /// ```
    pub fn build(self) -> Result<Arc<Server<T>>, Error> {
        let rng = rand::SystemRandom::new();
        Ok(Arc::new(Server {
            pure_branch: Arc::new(self.branch.purify()),
            additional: Arc::new(Additional {
                shared: self.shared,
                secret: Arc::new(Key::generate(hmac::HMAC_SHA256, &rng).map_err(|_| Error::Ring)?)
            }),
            log_string: Arc::new(self.log_string),
            max_connections: Arc::new(Semaphore::new(self.max_connections)),
            timeout: Arc::new(self.timeout),
            #[cfg(feature = "demon")]
            gate: Arc::new(self.gate.ok_or_else(|| Error::MissingGate)?)
        }))
    }
}

/// Http Server instance
///
/// The Server structure hosts all the information to successfully process each call
pub struct Server<T> {
    pure_branch: Arc<PureBranch<T>>,
    additional: Arc<Additional<T>>,
    log_string: Arc<Option<String>>,
    max_connections: Arc<Semaphore>,
    timeout: Arc<std::time::Duration>,
    #[cfg(feature = "demon")]
    gate: Arc<Gate>
}

impl<T: 'static + Sync + Send> Server<T> {
    // Short for ServerBuilder's `new` function.
    pub fn builder(branch: Branch<T>) -> ServerBuilder<T> {
        ServerBuilder::new(branch)
    }

    pub async fn run<S: AsRef<str>>(self: &Arc<Self>, socket: S) -> Result<(), Error> {
        let listener = TcpListener::bind(socket.as_ref()).await.map_err(|e| Error::Io(e))?;

        info!("Cataclysm ongoing \u{26c8}");
        // We need a fused future for the select macro
        tokio::select! {
            _ = async {
                loop {
                    // We lock the loop until one permit becomes available
                    self.max_connections.acquire().await.unwrap().forget();
                    
                    match listener.accept().await {
                        Ok((socket, addr)) => {
                            let server = Arc::clone(self);
                            
                            tokio::spawn(async move {
                                tokio::select! {
                                    res = server.dispatch(socket, addr) => match res {
                                        Ok(_) => (),
                                        Err(e) => {
                                            error!("{}", e);
                                        }
                                    },
                                    _ = tokio::time::sleep(*server.timeout) => {
                                        log::debug!("timeout for http response");
                                    }
                                }
                                // We set up back the permits
                                server.max_connections.add_permits(1);
                            });
                        },
                        Err(e) => {
                            error!("{}", e);
                        }
                    }
                }
            } => (),
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down server");
            }
        };
        Ok(())
    }

    /// Deals with the read part of the socket stream
    async fn dispatch_read(socket: &TcpStream) -> Result<Option<Vec<u8>>, Error> {
        let mut request_bytes = Vec::with_capacity(8192);
        let mut expected_length = None;
        let mut header_size = 0;
        let mut request = None;
        // First we read
        loop {
            socket.readable().await.map_err(|e| Error::Io(e))?;
            
            // being stored in the async task.
            let mut buf = [0; 8 * 1024];

            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match socket.try_read(&mut buf) {
                Ok(0) => {
                    break
                },
                Ok(n) => request_bytes.extend_from_slice(&buf[0..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if request.is_none() {
                        request = match Request::parse(request_bytes.clone()) {
                            Ok(r) => {
                                // We check if we need to give a continue 100
                                if r.headers.get("Expect").map(|h| h == "100-continue").unwrap_or(false) {
                                    Server::<T>::dispatch_write(&socket, Response::r#continue()).await?;
                                    continue;
                                }

                                // We check now if there is a content size hint
                                expected_length = r.headers.get("Content-Length").map(|v| v.parse::<usize>().ok()).flatten();
                                header_size = r.header_size;
                                Some(r)
                            },
                            Err(e) => {
                                log::debug!("{}", e);
                                Server::<T>::dispatch_write(&socket, Response::bad_request()).await?;
                                return Ok(None)
                            }
                        };
                    }

                    // And now we check if, given the hint, we need to act upon.
                    if let Some(expected_length) = &expected_length {
                        if *expected_length > request_bytes.len() - header_size {
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(Error::Io(e))
            }
        }
        Ok(Some(request_bytes))
    }

    async fn dispatch_write(socket: &TcpStream, mut response: Response) -> Result<(), Error> {
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

    async fn dispatch(self: &Arc<Self>, socket: TcpStream, addr: std::net::SocketAddr) -> Result<(), Error> {
        // let mut second_part = false;
        let request_bytes = match Server::<T>::dispatch_read(&socket).await? {
            Some(b) => b,
            None => return Ok(())
        };
        let mut request =match Request::parse(request_bytes.clone()) {
            Ok(r) => r,
            Err(e) => {
                log::debug!("{}", e);
                Server::<T>::dispatch_write(&socket, Response::bad_request()).await?;
                return Ok(())
            }
        };

        // The request could be an upgrade request for a websockets connection
        #[cfg(feature = "ws")]
        if request.headers.get("Upgrade").map(|v| v == "websocket").unwrap_or(false) && request.headers.get("Connection").map(|v| v == "Upgrade" || v == "keep-alive, Upgrade").unwrap_or(false) {
            if let Some(nonce) = request.headers.get("Sec-WebSocket-Key") {
                // According to RFC4122
                let nonce = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", nonce);
                let websocket_accept = base64::encode(ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, nonce.as_bytes()));

                // We check if there is a demon handler
                #[cfg(feature = "demon")]
                match self.pure_branch.websocket_demon_handler(request.path()) {
                    Some(handler) => {
                        let response = Response::switching_protocols().header("Upgrade", "websocket").header("Connection", "Upgrade").header("Sec-WebSocket-Accept", websocket_accept);
                        Server::<T>::dispatch_write(&socket, response).await?;
                        let (owned_read, owned_write) = socket.into_split();
                        let web_socket_writer = WebSocketWriter::new(owned_write);
                        handler(web_socket_writer, (*self.gate).clone(), owned_read).await?;
                        return Ok(())
                    },
                    None => {
                        // Nothing found, we go to the normal websocket handler
                    }
                }

                // We request a websocket handler
                match self.pure_branch.websocket_handler(request.path()) {
                    Some(handler) => {
                        let response = Response::switching_protocols().header("Upgrade", "websocket").header("Connection", "Upgrade").header("Sec-WebSocket-Accept", websocket_accept);
                        Server::<T>::dispatch_write(&socket, response).await?;
                        let (owned_read, owned_write) = socket.into_split();
                        let web_socket_writer = WebSocketWriter::new(owned_write);
                        WebSocketThread::spawn(owned_read, handler(web_socket_writer).await);
                    },
                    None => {
                        Server::<T>::dispatch_write(&socket, Response::not_found()).await?;
                    }
                }
            } else {
                Server::<T>::dispatch_write(&socket, Response::bad_request()).await?;
            }
            return Ok(())
        }

        request.addr = Some(addr);
        
        // The method will take the request, and modify particularly the "variable count" variable
        let response = match self.pure_branch.pipeline(&mut request) {
            Some(pipeline) => {
                match pipeline {
                    Pipeline::Layer(func, pipeline_layer) => func(request.clone(), pipeline_layer, self.additional.clone()),
                    Pipeline::Core(core_fn) => core_fn(request.clone(), self.additional.clone())
                }.await
            },
            None => Response::not_found()
        };
        if let Some(log_string) = &*self.log_string {
            info!("{}", log_string.replace("%M", request.method.to_str()).replace("%P", &request.path()).replace("%A", &format!("{}", addr)).replace("%S", &format!("{}", response.status.0)));
        }
        Server::<T>::dispatch_write(&socket, response).await?;
            //socket.shutdown();
        Ok(())
    }
}