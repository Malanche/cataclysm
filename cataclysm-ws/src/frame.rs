use crate::{FrameParseError, Message};

/// Frame structure from websockets connection
pub struct Frame {
    inner_op_code: u8,
    masking_key: Option<u32>,
    /// Inner message
    pub message: Message
}

impl Frame {
    /// FIN RSV bytes
    pub const FIN_RSV: u8 = 0x80;

    // The operation codes are the last 4 bytes of the u8

    /// Operation code for a continuation request
    pub const OP_CODE_CONTINUATION: u8 = 0x00;
    /// Operation code for a text message
    pub const OP_CODE_TEXT: u8 = 0x01;
    /// Operation code for a binary message
    pub const OP_CODE_BINARY: u8 = 0x02;
    /// Operation code for a close request message
    pub const OP_CODE_CLOSE: u8 = 0x08;
    /// Operation code for a ping message
    pub const OP_CODE_PING: u8 = 0x09;
    /// Operation code for a pong message
    pub const OP_CODE_PONG: u8 = 0x0A;

    /// Returns the OP CODE of the frame as a u8, where the last 4 bits contain the OP CODE
    pub fn op_code(&self) -> u8 {
        self.inner_op_code
    }

    /// Attempts to parse a frame from a stream of bytes
    pub fn parse<A: AsRef<[u8]>>(content: A) -> Result<Frame, FrameParseError> {
        let candidate = content.as_ref();

        if candidate.is_empty() {
            // Not enough bytes to even read a possible FIN_RSV + OP_CODE, and prevent panics
            return Err(FrameParseError::NullContent);
        }

        // The or operation needs to put ones on the first 4 bits, if candidate[0] matches FIN_RSV
        if ((candidate[0] ^ !Frame::FIN_RSV) >> 4) != 0x0f {
            return Err(FrameParseError::WrongFinRSV);
        }

        // We extract the minimum length, removing the masking key
        let min_length = candidate[1] & (!0x80);
        let (length, mut offset) = if min_length == 126 {
            if candidate.len() < 4 {
                return Err(FrameParseError::Malformed)
            }
            (u16::from_be_bytes([candidate[2], candidate[3]]) as usize, 4usize)
        } else if min_length == 127 {
            if candidate.len() < 10 {
                return Err(FrameParseError::Malformed)
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
        let mut payload = candidate.get(offset..offset+length).ok_or_else(|| FrameParseError::Incomplete)?.to_vec();
        if let Some(masking_key) = &masking_key {
            // We decode the content in case we have a masking key
            payload = payload.into_iter().enumerate().map(|(idx, v)| {
                // According to rfc 6455
                let j = idx % 4;
                v ^ masking_key[j]
            }).collect();
        }
        let message = match inner_op_code {
            Frame::OP_CODE_TEXT => Message::Text(String::from_utf8(payload).map_err(|e| FrameParseError::InvalidUtf8(e))?),
            Frame::OP_CODE_BINARY => Message::Binary(payload),
            Frame::OP_CODE_PING => Message::Ping,
            Frame::OP_CODE_PONG => Message::Pong,
            Frame::OP_CODE_CLOSE => Message::Close,
            _ => return Err(FrameParseError::UnsupportedOpCode)
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
        let message = Message::Text(payload);
        Frame {
            inner_op_code: Frame::OP_CODE_TEXT,
            masking_key: None,
            message
        }
    }

    /// Creates a binary frame
    pub fn binary<A: Into<Vec<u8>>>(binary: A) -> Frame {
        let payload = binary.into();
        let message = Message::Binary(payload);
        Frame {
            inner_op_code: Frame::OP_CODE_BINARY,
            masking_key: None,
            message
        }
    }

    /// Creates a close frame
    pub fn close() -> Frame {
        let masking_key = Some(rand::random::<u32>());
        Frame {
            inner_op_code: Frame::OP_CODE_CLOSE,
            masking_key,
            message: Message::Close
        }
    }

    /// Takes the frame and returns the contained message, if any
    pub fn get_message(&self) -> &Message {
        &self.message
    }

    /// Indicates if this frame is a closing frame
    pub fn is_close(&self) -> bool {
        self.inner_op_code == Frame::OP_CODE_CLOSE
    }
}

impl From<Frame> for Message {
    fn from(source: Frame) -> Message {
        source.message
    }
}

impl From<Frame> for Vec<u8> {
    fn from(source: Frame) -> Vec<u8> {
        let mut content = vec![Frame::FIN_RSV ^ source.inner_op_code];
        let mut payload: Vec<u8> = source.message.into();
        let payload_length = payload.len();
        if payload_length < 126 {
            content.push(payload_length as u8 | if source.masking_key.is_some() {0x80} else {0x00});
        } else if payload_length <= u16::MAX.into() /*65535*/{
            content.push(126u8 | if source.masking_key.is_some() {0x80} else {0x00});
            // And now we push the length as u16
            content.extend((payload_length as u16).to_be_bytes());
        } else {
            content.push(127u8 | if source.masking_key.is_some() {0x80} else {0x00});
            // And now we push the length as u16
            content.extend((payload_length as u64).to_be_bytes());
        }

        if let Some(masking_key) = &source.masking_key {
            let masking_bytes = masking_key.to_be_bytes();
            payload = payload.into_iter().enumerate().map(|(idx, v)| {
                // According to rfc 6455
                let j = idx % 4;
                v ^ masking_bytes[j]
            }).collect();
            // We add the masking key
            content.extend(masking_bytes);
        }
        if !payload.is_empty() {
            content.extend(payload);
        }
        
        content
    }
}