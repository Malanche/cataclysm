use crate::{Error, ws::Message};

pub struct Frame {
    op_code: u8,
    masking_key: Option<u32>,
    pub(crate) message: Message
}

impl Frame {
    pub const FIN_RSV: u8 = 0x80;
    pub const OP_CODE_CONTINUATION: u8 = 0x00;
    pub const OP_CODE_TEXT: u8 = 0x01;
    pub const OP_CODE_BINARY: u8 = 0x02;
    pub const OP_CODE_CLOSE: u8 = 0x08;
    pub const OP_CODE_PING: u8 = 0x09;
    pub const OP_CODE_PONG: u8 = 0x0A;

    /// Parses a frame from a stream of bytes
    pub fn parse<A: AsRef<[u8]>>(content: A) -> Result<Frame, Error> {
        let candidate = content.as_ref();
        if candidate.len() < 2 {
            return Err(Error::Incomplete)
        }
        // We check if the first two bytes are correct
        if ((candidate[0] ^ !Frame::FIN_RSV) >> 4) == 0x08 {
            return Err(Error::Parse(format!("Malformed message")))
        }
        // We extract the minimum length, removing the masking key
        let min_length = candidate[1] & (!0x80);
        let (length, mut offset) = if min_length == 126 {
            panic!("Too long!");
        } else if min_length == 127 {
            panic!("Way too long!");
        } else {
            (min_length as usize, 2usize)
        };
        // Now, the masking key, if any
        let masking_key = if 0x80 == (candidate[1] & 0x80) {
            offset += 4;
            // Now, we have to add 0, 1, 2 and 3 respectively, without the 4 that we just added.
            Some([candidate[offset - 4], candidate[offset - 3], candidate[offset - 2], candidate[offset - 1]])
        } else {
            None
        };
        // Now we read the operation code
        let op_code = (candidate[0] << 4) >> 4;
        let mut payload = candidate.get(offset..offset+length).ok_or_else(|| Error::Incomplete)?.to_vec();
        if let Some(masking_key) = &masking_key {
            payload = payload.into_iter().enumerate().map(|(idx, v)| {
                // According to rfc 6455
                let j = idx % 4;
                v ^ masking_key[j]
            }).collect();
        }
        let message = match op_code {
            Frame::OP_CODE_TEXT => Message::Text(String::from_utf8(payload).map_err(|e| Error::Parse(format!("{}", e)))?),
            _ => panic!("Not supported")
        };

        Ok(Frame {
            op_code,
            masking_key: masking_key.map(u32::from_be_bytes),
            message
        })
    }

    /// Creates a text frame
    pub fn text<A: Into<String>>(text: A) -> Frame {
        let payload = text.into();
        let message = Message::Text(payload);
        Frame {
            op_code: Frame::OP_CODE_TEXT,
            masking_key: None,
            message
        }
    }

    pub fn bytes(self) -> Vec<u8> {
        let mut content = vec![Frame::FIN_RSV ^ self.op_code];
        let mut payload = self.message.to_bytes();
        let payload_length = payload.len();
        if payload_length < 126 {
            content.push(payload_length as u8 | if self.masking_key.is_some() {0x80} else {0x00});
        } else {
            panic!("Oh noes!");
        }

        if let Some(masking_key) = &self.masking_key {
            let masking_bytes = masking_key.to_be_bytes();
            payload = payload.into_iter().enumerate().map(|(idx, v)| {
                // According to rfc 6455
                let j = idx % 4;
                v ^ masking_bytes[j]
            }).collect();
            content.extend(masking_bytes);
        }
        
        content.extend(payload);
        
        content
    }
}