pub use self::callback::{Callback, CoreFn, LayerFn, Pipeline};
#[cfg(feature = "stream")]
pub use self::callback::{StreamCallback, HandlerFn};
pub(crate) mod callback;

pub use self::extractor::Extractor;
mod extractor;

/*
// The metafunctions module contains the implementation of an emulation of "variadic" functions in Rust.
//
// It works by using macros and generic parameters (which can be found in the `callback.rs` file) and
// also using traits (found in the `extractor.rs` file). The last important part is located in `http/method.rs`,
// as closures are used to wrap functions with multiple parameters. Simple explanations are found in said files.
*/