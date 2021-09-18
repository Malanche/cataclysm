use crate::{Error, Additional, Extractor, http::Request};
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Json extractor
///
/// Allows to use a structure that implements `DeserializeOwned` to extract information as json from the body of a request
///
/// ```rust, no_run
/// use cataclysm::http::{Response, Json};
/// use serde::{Deserialize};
///
/// #[derive(Deserialize, Debug)]
/// struct BodyParams {
///     name: String,
///     last_name: Option<String>
/// }
/// 
/// async fn check_body(json: Json<BodyParams>) -> Response {
///     log::info!("Http call containing {:?}", json.into_inner());
///     Response::ok()
/// }
/// ```
///
/// Deserialization error will result always in a bad request response
pub struct Json<J>(pub J);

impl<J> Json<J> {
    /// Retrieves the inner instance of the generic type
    pub fn into_inner(self) -> J {
        self.0
    }
}

impl<T: Sync, J: 'static + DeserializeOwned + Send + Sync> Extractor<T> for Json<J> {
    fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if req.headers.get("Content-Type").map(|val| val == "application/json").unwrap_or(false) {
            match String::from_utf8(req.content.clone()) {
                Ok(body) => {
                    serde_json::from_str::<J>(&body)
                        .map(|j| Json(j))
                        .map_err(|e| Error::ExtractionBR(format!("json deserialization failure, {}", e)))
                },
                Err(e) => {
                    Err(Error::ExtractionBR(format!("body encoding error, {}", e)))
                }
            }
        } else {
            Err(Error::ExtractionBR(format!("missing header Content-Type, or incorrect content from it")))
        }
    }
}