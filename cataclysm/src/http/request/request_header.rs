use std::collections::HashMap;
use crate::{Error, http::Method};
use super::Request;
use url::Url;

#[derive(Clone, Debug)]
pub struct RequestHeader {
    /// Method that the request used
    pub(crate) method: Method,
    /// Route that the user requested
    pub(crate) url: Url,
    /// Variable positions, if any (set by the pure branch)
    pub(crate) variable_indices: Vec<usize>,
    /// How deep in the tree this endpoint finds itself (set by the pure branch)
    pub(crate) depth: usize,
    /// Header map for the request
    pub headers: HashMap<String, Vec<String>>,
    /// Header size in bytes
    #[allow(unused)]
    pub(crate) header_size: usize,
    /// Address from the request
    pub(crate) addr: std::net::SocketAddr,
}

impl RequestHeader {
    /// Extracts a header from an http request
    ///
    /// This method consumes the bytes corresponding to the header, and leaves the source with the remaining bytes (the content or body of the http call)
    pub(crate) fn parse(source: &mut Vec<u8>, addr: std::net::SocketAddr) -> Result<RequestHeader, Error> {
        // http call should have at least 3 bytes. For sure
        let (one, two) = (source.iter(), source.iter().skip(2));

        // We need to find the index where the header ends
        let mut split_index = None;
        for (idx, (a, b)) in one.zip(two).enumerate() {
            if a==b && b==&b'\n' && idx > 0 && source[idx-1] == b'\r' && source[idx+1] == b'\r' {
                split_index = Some(idx);
                break;
            }
        }

        // There must be one, in a properly formatted header
        let split_index = split_index.ok_or(Error::Parse(format!("no end of header was found")))?;

        // The minus one is a safe operation, due to the upper for loop
        let mut header: Vec<_> = source.drain((split_index - 1)..).collect();
        // We have to remove the `\r\n\r\n` that is at the beginning of the remaining bytes
        header.drain(..4);

        // We swap the source and the remaining part
        std::mem::swap(&mut header, source);

        // The request header needs to be a string, and we count the 4 ending symbols of the header
        let header_size = header.len() + 4;
        let header_string = String::from_utf8(header).map_err(|e| Error::Parse(format!("{}", e)))?;

        let mut lines = header_string.split("\r\n");
        let first_line = lines.next().ok_or(Error::Parse("request has no first line".into()))?;
        let tokens = first_line.split(" ").collect::<Vec<_>>();
        let (method, path, version) = if tokens.len() != 3 {
            return Err(Error::Parse("request's first has incorrect format".into()));
        } else {
            (
                tokens[0].into(), // We force the method conversion
                tokens[1],
                tokens[2]
            )
        };
        // We parse the remaining headers
        let mut headers = HashMap::new();
        for line in lines {
            let idx = line.find(":").ok_or(Error::Parse(format!("corrupted header missing colon")))?;
            let (key, value) = line.split_at(idx);
            let (key, value) = (key.to_string(), value.trim_start_matches(": ").trim_end().to_string());
            headers.entry(key).or_insert_with(|| Vec::new()).push(value);
        }

        if !version.starts_with("HTTP") {
            return Err(Error::Parse("unsupported protocol".into()))
        }
        // And we construct the request
        let host = headers.get("Host").map(|h| h.get(0).map(|v| &v[..])).flatten().unwrap_or_else(|| "missing.host");
        let url = Url::parse(&format!("http://{}{}", host, path)).map_err(Error::Url)?;
        
        Ok(RequestHeader {
            method,
            url,
            variable_indices: vec![],
            depth: 0,
            headers,
            header_size,
            addr
        })
    }

    /// Indicates if a keep alive request has been sent
    pub(crate) fn requests_keep_alive(&self) -> bool {
        self.headers.get("Connection").map(|values| values.into_iter().find(|v| *v == "keep-alive")).flatten().is_some()
    }

    /// Generates a full request with content
    pub(crate) fn content(self, content: Vec<u8>) -> Request {
        Request {
            header: self,
            content
        }
    }
}