use std::collections::HashMap;
use crate::{Error, http::Method};

#[derive(Clone)]
pub struct Request {
    /// Method that the request used
    pub(crate) method: Method,
    /// Route that the user requested
    pub(crate) path: String,
    /// Variable positions, if any (this is used by the Path structure)
    pub(crate) variable_indices: Vec<usize>,
    /// headers from the request
    pub headers: HashMap<String, String>,
    /// Address from the request
    pub(crate) addr: Option<std::net::SocketAddr>,
    pub(crate) content: Vec<u8>
}

impl Request {
    pub fn parse(mut source: Vec<u8>) -> Result<Request, Error> {
        // http call should have at least 3 bytes. For sure
        let (one, two) = (source.iter(), source.iter().skip(2));

        let mut split_index = None;
        for (idx, (a, b)) in one.zip(two).enumerate() {
            if a==b && b==&b'\n' {
                split_index = Some(idx);
                break;
            }
        }

        let split_index = split_index.ok_or(Error::Parse(format!("no end of header was found")))?;

        let content: Vec<_> = source.drain((split_index)..).collect();
        //source.truncate(split_index);
        // Cambiamos esto a una cadena
        let request_string = String::from_utf8(source).map_err(|e| Error::Parse(format!("{}", e)))?;
        let mut lines = request_string.split("\n");
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
            headers,
            addr: None,
            content
        })
    }
}