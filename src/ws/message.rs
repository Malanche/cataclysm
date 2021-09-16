/// Message structure contained in a frame
pub enum Message {
    Text(String),
    Bytes(Vec<u8>),
    Ping,
    Pong
}

impl Message {
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            Message::Text(content) => content.into(),
            Message::Bytes(content) => content,
            Message::Ping => vec![],
            Message::Pong => vec![]
        }
    }
}