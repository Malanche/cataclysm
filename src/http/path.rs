use crate::{Extractor, Error, http::Request, branch::Tokenizable};
use std::str::FromStr;

pub struct Path<T>(pub T);

impl<T> Path<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

macro_rules! tuple_path {
    (($struct_name:ident, $struct_error:ident, $index:tt)) => {
        impl<$struct_error: std::error::Error, $struct_name: 'static + FromStr<Err = $struct_error> + Send> Extractor for Path<($struct_name,)> {
            fn extract(req: &Request) -> Result<Self, Error> {
                let token = *req.path.tokenize().iter().nth(*req.variable_indices.get(0).ok_or_else(|| Error::ExtractionSE(format!("Not enough elements")))?).ok_or_else(|| Error::ExtractionSE(format!("Not enough elements")))?;
                Ok(Path(($struct_name::from_str(token).map_err(|e| Error::ExtractionBR(format!("{}", e)))?, )))
            }
        }
    };
    ($(($struct_name:ident, $struct_error:ident, $index:tt)),+) => {
        impl<$($struct_error: std::error::Error, $struct_name: 'static + FromStr<Err = $struct_error> + Send),+> Extractor for Path<($($struct_name),+)> {
            fn extract(req: &Request) -> Result<Self, Error> {
                let trimmed_trail = req.path.trim_start_matches("/");
                let tokens = trimmed_trail.tokenize();

                Ok(Path(($({
                    let token = tokens.get(
                        *req.variable_indices.get($index).ok_or_else(|| Error::ExtractionSE(format!("There are more path extractors than parameters in the path")))?
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