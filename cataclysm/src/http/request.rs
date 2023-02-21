use std::collections::HashMap;
use crate::{Error, http::Method};
use url::Url;

/// Contains the data from an http request.
#[derive(Clone)]
pub struct Request {
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
    pub(crate) header_size: usize,
    /// Address from the request
    pub(crate) addr: std::net::SocketAddr,
    pub(crate) content: Vec<u8>
}

impl Request {
    /// Returns the [Url](https://docs.rs/url/latest/url/struct.Url.html) object for this request
    pub fn method(&self) -> &Method {
        &self.method
    }
    
    /// Returns the [Url](https://docs.rs/url/latest/url/struct.Url.html) object for this request
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns the IP address from which this call has been made
    pub fn address(&self) -> std::net::SocketAddr {
        self.addr
    }

    /// Returns the body as bytes of the content
    pub fn body(&self) -> &Vec<u8> {
        &self.content
    }

    pub(crate) fn parse(mut source: Vec<u8>, addr: std::net::SocketAddr) -> Result<Request, Error> {
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
        //let _version = tokens[2];
        // Parse following lines
        Ok(Request {
            method,
            url,
            variable_indices: vec![],
            depth: 0,
            headers,
            header_size,
            addr,
            content
        })
    }
}

pub struct BasicRequest {
    /// Method that the request used
    method: Method,
    /// Route that the user requested
    url: Url,
    /// Header map for the request
    headers: HashMap<String, Vec<String>>,
    /// Content, if any
    content: Option<Vec<u8>>
}

impl BasicRequest {
    /// Creates a new basic request
    pub fn new<A: AsRef<str>>(method: Method, url: A) -> Result<BasicRequest, Error> {
        Ok(BasicRequest {
            method,
            url: Url::parse(url.as_ref()).map_err(Error::Url)?,
            headers: HashMap::new(),
            content: None
        })
    }

    /// Sets up a header to the request
    pub fn header<A: Into<String>, B: Into<String>>(mut self, key: A, value: B) -> Self {
        self.headers.entry(key.into()).or_insert_with(|| Vec::new()).push(value.into());
        self
    }

    /// Sets a content to the basic request
    pub fn content<A: Into<Vec<u8>>>(mut self, content: A) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Serializes the request
    pub fn serialize(&self) -> Vec<u8> {
        let mut content = String::new();
        // First line
        content += &format!("{} {} HTTP/1.1\r\n", self.method, self.url.path());
        for (header_name, header_contents) in &self.headers {
            for header_content in header_contents {
                content += &format!("{}: {}\r\n", header_name, header_content);
            }
        }

        content += "\r\n";
        let mut content = content.into_bytes();
        if let Some(final_content) = &self.content {
            content.extend_from_slice(final_content);
        }

        content
    }
}