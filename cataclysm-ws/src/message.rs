/// Message structure contained in a frame
pub enum Message {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(Vec<u8>),
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

    /// Geneates an instances of the [Message::Binary](Message::Binary) variant
    pub fn ping<A: Into<Vec<u8>>>(payload: A) -> Message {
        Message::Ping(payload.into())
    }

    /// Geneates an instances of the [Message::Binary](Message::Binary) variant
    pub fn pong<A: Into<Vec<u8>>>(payload: A) -> Message {
        Message::Pong(payload.into())
    }

    /// Indicates if the variant equates de [Message::Close](Message::Close) variant
    pub fn is_close(&self) -> bool {
        matches!(&self, Message::Close)
    }

    /// Indicates if the variant equates de [Message::Ping](Message::Ping) variant
    pub fn is_ping(&self) -> bool {
        matches!(&self, Message::Ping(_))
    }

    /// Indicates if the variant equates de [Message::Pong](Message::Pong) variant
    pub fn is_pong(&self) -> bool {
        matches!(&self, Message::Pong(_))
    }
}

impl From<Message> for Vec<u8> {
    fn from(source: Message) -> Vec<u8> {
        match source {
            Message::Text(content) => content.into(),
            Message::Binary(content) => content,
            Message::Ping(content) => content,
            Message::Pong(content) => content,
            Message::Close => vec![]
        }
    }
}