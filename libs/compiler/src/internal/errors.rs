use std::fmt::{self, Display};

use napi::bindgen_prelude::Result as NapiResult;
use napi::{Error as NapiError, Status};

/// Canonical error type used by the Rust-facing API surface.
#[derive(Debug, Clone)]
pub struct Error {
  message: String,
}

impl Error {
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }

  pub fn with_context(context: impl AsRef<str>, cause: impl Display) -> Self {
    let mut message = context.as_ref().to_owned();
    if !message.ends_with(':') {
      message.push(':');
    }
    message.push(' ');
    message.push_str(&cause.to_string());
    Self { message }
  }

  pub fn message(&self) -> &str {
    &self.message
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl std::error::Error for Error {}

impl From<Error> for NapiError {
  fn from(err: Error) -> Self {
    NapiError::new(Status::GenericFailure, err.message)
  }
}

impl From<NapiError> for Error {
  fn from(err: NapiError) -> Self {
    Error::new(err.to_string())
  }
}

/// Result alias bound to [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Annotate an error from `result` with `context`, returning the shared [`Result`] type.
pub fn map_err_with_context<T, E>(
  result: std::result::Result<T, E>,
  context: impl AsRef<str>,
) -> Result<T>
where
  E: Display,
{
  result.map_err(|err| Error::with_context(context, err))
}

/// Convert a [`Result`] into a `napi::Result`.
pub fn to_napi_result<T>(result: Result<T>) -> NapiResult<T> {
  result.map_err(Into::into)
}

/// Create a `napi::Error` with the provided message.
pub(crate) fn napi_error(message: impl Into<String>) -> NapiError {
  NapiError::new(Status::GenericFailure, message.into())
}

/// Map an errorful result into a `napi::Result`, annotating the provided context
/// when the error is propagated.
pub(crate) fn map_napi_error<T, E>(
  result: std::result::Result<T, E>,
  context: &str,
) -> NapiResult<T>
where
  E: Display,
{
  result.map_err(|err| napi_error(format!("{context}: {err}")))
}
