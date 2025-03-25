use crate::Error;

/// SessionParser trait for serializing and deserializing session content
///
/// The crate provides a json implementaton, [JsonSessionParser](super::JsonSessionParser). It should be straight forward to implement other parsers (like url-encoding, for example)
pub trait SessionParser<T> {
    /// Function to be called when a valid `String` has been obtained from [SessionCreator](crate::session::SessionCreator).
    fn from_str(source: String) -> Result<T, Error>;
    /// Function to call when a valid `String` is to be sent to [SessionCreator](crate::session::SessionCreator).
    fn to_string(source: T) -> Result<String, Error>;
}