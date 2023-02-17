/// Message structure contained in a frame
pub enum Message {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping,
    /// Pong message
    Pong,
    /// Close message
    Close
}

impl Message {
    /// Geneates an instances of the [Message::Text](Message::Text) variant
    pub fn text<A: Into<String>>(text: A) -> Message {
        Message::Text(text.into())
    }

    /// Geneates an instances of the [Message::Binary](Message::Binary) variant
    pub fn binary<A: Into<Vec<u8>>>(bytes: A) -> Message {
        Message::Binary(bytes.into())
    }

    /// Indicates if the variant equates de [Message::Close](Message::Close) variant
    pub fn is_close(&self) -> bool {
        matches!(&self, Message::Close)
    }

    /// Indicates if the variant equates de [Message::Ping](Message::Ping) variant
    pub fn is_ping(&self) -> bool {
        matches!(&self, Message::Close)
    }
}

impl From<Message> for Vec<u8> {
    fn from(source: Message) -> Vec<u8> {
        match source {
            Message::Text(content) => content.into(),
            Message::Binary(content) => content,
            Message::Ping => vec![],
            Message::Pong => vec![],
            Message::Close => vec![]
        }
    }
}