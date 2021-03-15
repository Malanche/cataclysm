use std::collections::HashMap;
use crate::{Error, http::Method};

pub struct Request {
    pub(crate) method: Method,
    pub(crate) path: String,
    headers: HashMap<String, String>,
    content: Option<Vec<u8>>
}

impl Request {
    pub fn parse(source: Vec<u8>) -> Result<Request, Error> {
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
        let version = tokens[2];
        // Parse following lines
        for _line in lines {
            
        }
        Ok(Request {
            method,
            path,
            headers: HashMap::new(),
            content: None
        })
    }
}