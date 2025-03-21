use std::collections::HashMap;
use crate::{Error, http::Method};
use url::Url;

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
        let mut path_with_query = self.url.path().to_string();
        if let Some(query) = self.url.query() {
            path_with_query += &format!("?{}", query);
        }
        content += &format!("{} {} HTTP/1.1\r\n", self.method, path_with_query);
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