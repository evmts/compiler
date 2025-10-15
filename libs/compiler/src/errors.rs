use napi::bindgen_prelude::Result;
use napi::{Error, Status};

/// Create a GenericFailure `napi::Error` with the provided message.
pub(crate) fn napi_error(message: impl Into<String>) -> Error {
  Error::new(Status::GenericFailure, message.into())
}

/// Map an errorful result into a `napi::Result`, annotating the provided context
/// when the error is propagated.
pub(crate) fn map_napi_error<T, E>(result: std::result::Result<T, E>, context: &str) -> Result<T>
where
  E: std::fmt::Display,
{
  result.map_err(|err| napi_error(format!("{context}: {err}")))
}
