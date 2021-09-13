use std::collections::HashMap;
use crate::{Error, http::Method};

/// Contains the data from an http request.
#[derive(Clone)]
pub struct Request {
    /// Method that the request used
    pub(crate) method: Method,
    /// Route that the user requested
    pub(crate) path: String,
    /// Variable positions, if any (set by the pure branch)
    pub(crate) variable_indices: Vec<usize>,
    /// How deep in the tree this endpoint finds itself (set by the pure branch)
    pub(crate) depth: usize,
    /// headers from the request
    pub headers: HashMap<String, String>,
    /// Header size in bytes
    pub(crate) header_size: usize,
    /// Address from the request
    pub(crate) addr: Option<std::net::SocketAddr>,
    pub(crate) content: Vec<u8>
}

impl Request {
    pub(crate) fn parse(mut source: Vec<u8>) -> Result<Request, Error> {
        // http call should have at least 3 bytes. For sure
        let (one, two) = (source.iter(), source.iter().skip(2));

        let mut split_index = None;
        for (idx, (a, b)) in one.zip(two).enumerate() {
            if a==b && b==&b'\n' && idx > 0 && source[idx-1] == b'\r' && source[idx+1] == b'\r' {
                split_index = Some(idx);
                break;
            }
        }

        let split_index = split_index.ok_or(Error::Parse(format!("no end of header was found")))?;

        // The minus one is a safe operation, due to the upper for loop
        let mut content: Vec<_> = source.drain((split_index - 1)..).collect();
        // We have to remove the `\r\n\r\n` that is at the beginning of the remaining bytes
        content.drain(..4);
        // The request header needs to be a string
        let header_size = source.len() + 4;
        let request_string = String::from_utf8(source).map_err(|e| Error::Parse(format!("{}", e)))?;
        let mut lines = request_string.split("\r\n");
        let first_line = lines.next().ok_or(Error::Parse("request has no first line".into()))?;
        let tokens = first_line.split(" ").collect::<Vec<_>>();
        if tokens.len() < 3 {
            return Err(Error::Parse("request's first has incorrect format".into()));
        }
        let method = Method::from_str(tokens[0]).ok_or(Error::Parse("method does not seem to exist".into()))?;
        let path = tokens[1].to_string();
        let _version = tokens[2];
        // Parse following lines
        let mut headers = HashMap::new();
        for line in lines {
            let idx = line.find(":").ok_or(Error::Parse(format!("corrupted header missing colon")))?;
            let (key, value) = line.split_at(idx);
            let (key, value) = (key.to_string(), value.trim_start_matches(": ").trim_end().to_string());
            headers.insert(key, value);
        }
        Ok(Request {
            method,
            path,
            variable_indices: vec![],
            depth: 0,
            headers,
            header_size,
            addr: None,
            content
        })
    }
}