pub use cataclysm_ws::{Error as WSError, WebSocketStream, WebSocketReader, WebSocketWriter, WebSocketThread, Message, Frame};
use crate::{
    Stream,
    Error,
    http::{Request, Response}
};
use base64::{Engine, engine::general_purpose};

pub struct WebSocketHandshake {
    protocol: Option<String>
}

impl WebSocketHandshake {
    pub fn new() -> WebSocketHandshake {
        WebSocketHandshake {
            protocol: None
        }
    }

    pub fn protocol<A: Into<String>>(mut self, protocol: A) -> WebSocketHandshake {
        self.protocol = Some(protocol.into());
        self
    }

    pub async fn perform(self, stream: Stream, request: Request) -> Result<WebSocketStream, Error> {
        if request.headers.get("Upgrade").map(|u| u.get(0).map(|v| v == "websocket")).flatten().unwrap_or(false) && request.headers.get("Connection").map(|c| c.get(0).map(|v| v == "Upgrade" || v == "keep-alive, Upgrade")).flatten().unwrap_or(false) {
            if let Some(nonce) = request.headers.get("Sec-WebSocket-Key").map(|wsk| wsk.get(0)).flatten() {
                // According to RFC4122
                let nonce = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", nonce);
                let websocket_accept = general_purpose::STANDARD.encode(ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, nonce.as_bytes()));

                let mut response = Response::switching_protocols()
                    .header("Upgrade", "websocket")
                    .header("Connection", "Upgrade");

                if let Some(protocol) = self.protocol {
                    if let Some(available_protocols) = request.headers.get("Sec-WebSocket-Protocol") {
                        let mut found = false;
                        for header in available_protocols {
                            if header.split(",").map(|v| v.trim()).find(|v| *v == protocol).is_some() {
                                found = true;
                                break;
                            }
                        }

                        if found {
                            response = response.header("Sec-WebSocket-Protocol", protocol);
                        } else {
                            stream.response(Response::bad_request()).await?;
                            return Err(Error::custom("unsupported protocol for websockets exchange"));
                        }
                    } else {
                        stream.response(Response::bad_request()).await?;
                        return Err(Error::custom("missing Sec-WebSocket-Protocol header"));
                    }
                }

                response = response.header("Sec-WebSocket-Accept", websocket_accept);

                stream.response(response).await?;
                Ok(WebSocketStream::from_tcp_stream_unchecked(stream.into()))
            } else {
                stream.response(Response::bad_request()).await?;
                Err(Error::custom("nonce does not exist in websocket handshake"))
            }
        } else {
            Err(Error::custom("missing headers or headers with incorrect values"))
        }
    }
}