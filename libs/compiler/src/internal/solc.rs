use std::sync::{Mutex, OnceLock};

use semver::Version;

use foundry_compilers::solc::{Solc, SolcLanguage};
use napi::bindgen_prelude::Result;
use napi::{bindgen_prelude::AsyncTask, Env, Task};

use super::errors::{map_napi_error, napi_error};

pub(crate) const DEFAULT_SOLC_VERSION: &str = "0.8.30";

pub(crate) fn default_language() -> SolcLanguage {
  SolcLanguage::Solidity
}

pub(crate) fn parse_version(version: &str) -> Result<Version> {
  let trimmed = version.trim().trim_start_matches('v');
  map_napi_error(Version::parse(trimmed), "Failed to parse solc version")
}

pub(crate) fn default_version() -> Result<Version> {
  parse_version(DEFAULT_SOLC_VERSION)
}

pub(crate) fn ensure_installed(version: &Version) -> Result<Solc> {
  if let Some(solc) = find_installed_version(version)? {
    return Ok(solc);
  }
  Err(napi_error(format!(
    "Solc {} is not installed. Call installSolcVersion first.",
    version
  )))
}

pub(crate) fn find_installed_version(version: &Version) -> Result<Option<Solc>> {
  let maybe_solc = map_napi_error(
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

  fn compute(&mut self) -> Result<Self::Output> {
    let _guard = install_mutex()
      .lock()
      .map_err(|err| napi_error(format!("Solc install mutex poisoned: {err}")))?;

    if find_installed_version(&self.version)?.is_some() {
      return Ok(());
    }
    map_napi_error(
      Solc::blocking_install(&self.version),
      "Failed to install solc version",
    )
    .map(|_| ())
  }

  fn resolve(&mut self, _env: Env, _output: Self::Output) -> Result<Self::JsValue> {
    Ok(())
  }
}
