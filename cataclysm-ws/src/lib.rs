//! ## Auxiliary crate for websocket support in cataclysm
//!
//! All structures of this crate are exported on [cataclysm](https://docs.rs/cataclysm)

pub use self::web_socket_stream::WebSocketStream;
pub use self::web_socket_reader::{WebSocketReader, WebSocketCustomChild};
pub use self::web_socket_writer::WebSocketWriter;
pub use self::web_socket_thread::WebSocketThread;
pub use self::frame::Frame;
pub use self::message::Message;
pub use self::error::{Error, FrameParseError};

mod web_socket_stream;
mod web_socket_reader;
mod web_socket_writer;
mod web_socket_thread;
mod frame;
mod message;
mod error;

pub(crate) mod communication;