use crate::{Error, Additional, Extractor, http::Request};
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Query extractor
///
/// Allows to use a structure that implements `DeserializeOwned` to extract information easier from the query
///
/// ```rust, no_run
/// use cataclysm::http::{Response, Query};
/// use serde::{Deserialize};
///
/// #[derive(Deserialize)]
/// struct QueryParams {
///     name: String,
///     last_name: Option<String>
/// }
/// 
/// async fn check_query(query: Query<QueryParams>) -> Response {
///     log::info!("Http call from {}", query.into_inner().name);
///     Response::ok()
/// }
/// ```
///
/// Deserialization error will result always in a bad request response
pub struct Query<Q>(pub Q);

impl<Q> Query<Q> {
    /// Retrieves the inner instance of the generic type
    pub fn into_inner(self) -> Q {
        self.0
    }
}

impl<T: Sync, Q: 'static + DeserializeOwned + Send> Extractor<T> for Query<Q> {
    fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(query) = req.query() {
            serde_qs::from_str::<Q>(query)
        } else {
            // We will check if the Q could be deserialized from an empty string
            serde_qs::from_str::<Q>("")
        }.map(|q| Query(q)).map_err(|e| Error::ExtractionBR(format!("query deserialization failure, {}", e)))
    }
}