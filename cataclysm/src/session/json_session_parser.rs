use super::{SessionParser};
use crate::Error;
use serde::{Serialize, de::{DeserializeOwned}};

/// Json implementation of a [SessionParser].
pub struct JsonSessionParser;

impl <T: Serialize + DeserializeOwned> SessionParser<T> for JsonSessionParser {
    fn from_str(source: String) -> Result<T, Error> {
        Ok(serde_json::from_str(&source)?)
    }

    fn to_string(source: T) -> Result<String, Error> {
        Ok(serde_json::to_string(&source)?)
    }
}