use crate::{Error, Message};

/// Frame structure from websockets connection
pub struct Frame {
    inner_op_code: u8,
    masking_key: Option<u32>,
    /// Payload
    pub(crate) message: Option<Message>
}

impl Frame {
    pub const FIN_RSV: u8 = 0x80;
    pub const OP_CODE_CONTINUATION: u8 = 0x00;
    pub const OP_CODE_TEXT: u8 = 0x01;
    pub const OP_CODE_BINARY: u8 = 0x02;
    pub const OP_CODE_CLOSE: u8 = 0x08;
    pub const OP_CODE_PING: u8 = 0x09;
    pub const OP_CODE_PONG: u8 = 0x0A;

    /// Returns the OP CODE of the frame
    pub fn op_code(&self) -> u8 {
        self.inner_op_code
    }

    /// Attempts to parse a frame from a stream of bytes
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
            if candidate.len() < 4 {
                return Err(Error::Incomplete)
            }
            (u16::from_be_bytes([candidate[2], candidate[3]]) as usize, 4usize)
        } else if min_length == 127 {
            if candidate.len() < 10 {
                return Err(Error::Incomplete)
            }
            (u64::from_be_bytes([candidate[2], candidate[3], candidate[4], candidate[5], candidate[6], candidate[7], candidate[8], candidate[9]]) as usize, 10usize)
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
        let inner_op_code = (candidate[0] << 4) >> 4;
        let mut payload = candidate.get(offset..offset+length).ok_or_else(|| Error::Incomplete)?.to_vec();
        if let Some(masking_key) = &masking_key {
            payload = payload.into_iter().enumerate().map(|(idx, v)| {
                // According to rfc 6455
                let j = idx % 4;
                v ^ masking_key[j]
            }).collect();
        }
        let message = match inner_op_code {
            Frame::OP_CODE_TEXT => Some(Message::Text(String::from_utf8(payload).map_err(|e| Error::Parse(format!("{}", e)))?)),
            Frame::OP_CODE_CLOSE => None,
            Frame::OP_CODE_PING => None,
            Frame::OP_CODE_PONG => None,
            _ => panic!("Not supported")
        };

        Ok(Frame {
            inner_op_code,
            masking_key: masking_key.map(u32::from_be_bytes),
            message
        })
    }

    /// Creates a text frame
    pub fn text<A: Into<String>>(text: A) -> Frame {
        let payload = text.into();
        let message = Some(Message::Text(payload));
        Frame {
            inner_op_code: Frame::OP_CODE_TEXT,
            masking_key: None,
            message
        }
    }

    /// Returns the frame as a byte vector
    pub fn bytes(self) -> Vec<u8> {
        let mut content = vec![Frame::FIN_RSV ^ self.inner_op_code];
        let mut payload = self.message.map(|m| m.to_bytes());
        let payload_length = payload.iter().map(|m| m.len()).next().unwrap_or(0);
        if payload_length < 126 {
            content.push(payload_length as u8 | if self.masking_key.is_some() {0x80} else {0x00});
        } else if payload_length <= u16::MAX.into() /*65535*/{
            content.push(126u8 | if self.masking_key.is_some() {0x80} else {0x00});
            // And now we push the length as u16
            content.extend((payload_length as u16).to_be_bytes());
        } else {
            content.push(127u8 | if self.masking_key.is_some() {0x80} else {0x00});
            // And now we push the length as u16
            content.extend((payload_length as u64).to_be_bytes());
        }

        if let Some(masking_key) = &self.masking_key {
            let masking_bytes = masking_key.to_be_bytes();
            payload = payload.map(|b| b.into_iter().enumerate().map(|(idx, v)| {
                // According to rfc 6455
                let j = idx % 4;
                v ^ masking_bytes[j]
            }).collect());
            content.extend(masking_bytes);
        }
        if let Some(payload) = payload {
            content.extend(payload);
        }
        
        content
    }

    /// Takes the frame and returns the contained message, if any
    pub fn into_message(self) -> Option<Message> {
        self.message
    }
}