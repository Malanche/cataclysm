/// Message structure contained in a frame
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong
}

impl From<Message> for Vec<u8> {
    fn from(source: Message) -> Vec<u8> {
        match source {
            Message::Text(content) => content.into(),
            Message::Binary(content) => content,
            Message::Ping => vec![],
            Message::Pong => vec![]
        }
    }
}