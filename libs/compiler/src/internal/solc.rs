use std::sync::{Mutex, OnceLock};

use semver::Version;

use foundry_compilers::solc::{Solc, SolcLanguage};
use napi::{bindgen_prelude::AsyncTask, Env, Task};

use super::errors::{map_err_with_context, to_napi_result, Error, Result};

pub(crate) const DEFAULT_SOLC_VERSION: &str = "0.8.30";

pub(crate) fn default_language() -> SolcLanguage {
  SolcLanguage::Solidity
}

pub(crate) fn parse_version(version: &str) -> Result<Version> {
  let trimmed = version.trim().trim_start_matches('v');
  map_err_with_context(Version::parse(trimmed), "Failed to parse solc version")
}

pub(crate) fn default_version() -> Result<Version> {
  parse_version(DEFAULT_SOLC_VERSION)
}

pub(crate) fn ensure_installed(version: &Version) -> Result<Solc> {
  if let Some(solc) = find_installed_version(version)? {
    return Ok(solc);
  }
  Err(Error::new(format!(
    "Solc {} is not installed. Call installSolcVersion first.",
    version
  )))
}

pub(crate) fn find_installed_version(version: &Version) -> Result<Option<Solc>> {
  let maybe_solc = map_err_with_context(
    Solc::find_svm_installed_version(version),
    "Failed to inspect solc versions",
  )?;
  Ok(maybe_solc)
}

pub(crate) fn is_version_installed(version: &Version) -> Result<bool> {
  find_installed_version(version).map(|maybe| maybe.is_some())
}

pub(crate) fn install_async(version: Version) -> AsyncTask<InstallSolcTask> {
  AsyncTask::new(InstallSolcTask { version })
}

pub(crate) fn install_version(version: &Version) -> Result<()> {
  map_err_with_context(
    Solc::blocking_install(version).map(|_| ()),
    "Failed to install solc version",
  )
}

pub struct InstallSolcTask {
  pub(crate) version: Version,
}

fn install_mutex() -> &'static Mutex<()> {
  static INSTALL_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
  INSTALL_MUTEX.get_or_init(|| Mutex::new(()))
}

impl Task for InstallSolcTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let _guard = to_napi_result(
      install_mutex()
        .lock()
        .map_err(|err| Error::new(format!("Solc install mutex poisoned: {err}"))),
    )?;

    if to_napi_result(find_installed_version(&self.version))?.is_some() {
      return Ok(());
    }
    to_napi_result(map_err_with_context(
      Solc::blocking_install(&self.version),
      "Failed to install solc version",
    ))
    .map(|_| ())
  }

  fn resolve(&mut self, _env: Env, _output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_version_strips_whitespace_and_prefix() {
    let parsed = parse_version(" v0.8.11 ").expect("parse version");
    assert_eq!(parsed, Version::new(0, 8, 11));
  }

  #[test]
  fn parse_version_rejects_invalid_input() {
    let err = parse_version("abc").unwrap_err();
    assert!(err.to_string().contains("Failed to parse solc version"));
  }

  #[test]
  fn default_version_matches_constant() {
    let parsed = default_version().expect("default version");
    assert_eq!(parsed, Version::new(0, 8, 30));
  }

  #[test]
  fn ensure_installed_errors_for_missing_version() {
    let version = Version::new(0, 0, 0);
    let err = ensure_installed(&version).unwrap_err();
    assert!(
      err.to_string().contains("is not installed"),
      "unexpected message: {}",
      err
    );
  }

  #[test]
  fn find_installed_version_returns_none_for_missing_version() {
    let version = Version::new(0, 0, 0);
    let result = find_installed_version(&version).expect("find version");
    assert!(result.is_none());
  }

  #[test]
  fn is_version_installed_false_for_missing_version() {
    let version = Version::new(0, 0, 0);
    assert!(!is_version_installed(&version).expect("is installed"));
  }
}
