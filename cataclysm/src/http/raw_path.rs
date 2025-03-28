use crate::{Extractor, Error, http::Request, branch::Tokenizable, additional::Additional};
use std::str::FromStr;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};

/// Token extractor from the path from a request, that does not process percent encoding
///
/// The `RawPath` extractors allow for tuple extraction from a path with variable or regex components, leaving the match as it is without percent encoding processing
pub struct RawPath<T>(pub T);

// Convenience deref implementation
impl<P> Deref for RawPath<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> DerefMut for RawPath<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P> RawPath<P> {
    /// Retrieves the inner instance of the generic type
    pub fn into_inner(self) -> P {
        self.0
    }
}

macro_rules! tuple_path {
    (($struct_name:ident, $struct_error:ident, $index:tt)) => {
        impl<$struct_error: std::error::Error, $struct_name: 'static + FromStr<Err = $struct_error> + Send, T: Sync> Extractor<T> for RawPath<($struct_name,)> {
            fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
                let trimmed_trail = req.url().path().trim_start_matches("/");
                let token = *trimmed_trail.tokenize().iter().nth(*req.header.variable_indices.get(0).ok_or_else(|| Error::ExtractionSE(format!("Not enough elements")))?).ok_or_else(|| Error::ExtractionSE(format!("Not enough elements")))?;
                Ok(RawPath(($struct_name::from_str(token).map_err(|e| Error::ExtractionBR(format!("{}", e)))?, )))
            }
        }
    };
    ($(($struct_name:ident, $struct_error:ident, $index:tt)),+) => {
        impl<$($struct_error: std::error::Error, $struct_name: 'static + FromStr<Err = $struct_error> + Send),+, T: Sync> Extractor<T> for RawPath<($($struct_name),+)> {
            fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
                let trimmed_trail = req.url().path().trim_start_matches("/");
                let tokens = trimmed_trail.tokenize();

                Ok(RawPath(($({
                    let token = tokens.get(
                        *req.header.variable_indices.get($index).ok_or_else(|| Error::ExtractionSE(format!("There are more path extractors than parameters in the path")))?
                    ).ok_or_else(|| Error::ExtractionSE(format!("The path does not contain enough tokens to fill in the path extractors")))?;
                    $struct_name::from_str(
                        token
                    ).map_err(|e| Error::ExtractionBR(format!("failure for path extractor at location {}, token \"{}\", {}", $index, token, e)))?
                }),+ )))
            }
        }
    }
}

tuple_path!((A, Ae, 0));
tuple_path!((A, Ae, 0), (B, Be, 1));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3), (E, Ee, 4));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3), (E, Ee, 4), (F, Fe, 5));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3), (E, Ee, 4), (F, Fe, 5), (G, Ge, 6));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3), (E, Ee, 4), (F, Fe, 5), (G, Ge, 6), (H, He, 7));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3), (E, Ee, 4), (F, Fe, 5), (G, Ge, 6), (H, He, 7), (I, Ie, 8));
tuple_path!((A, Ae, 0), (B, Be, 1), (C, Ce, 2), (D, De, 3), (E, Ee, 4), (F, Fe, 5), (G, Ge, 6), (H, He, 7), (I, Ie, 8), (J, Je, 9));