use std::path::PathBuf;

use foundry_compilers::compilers::vyper::Vyper;

use crate::internal::errors::{Error, Result};

pub fn default_path() -> PathBuf {
  PathBuf::from("vyper")
}

pub fn ensure_installed(path: Option<PathBuf>) -> Result<Vyper> {
  let candidate = path.unwrap_or_else(default_path);
  Vyper::new(candidate.clone()).map_err(|err| {
    Error::new(format!(
      "Failed to initialise Vyper compiler at {}: {err}. Ensure `vyper` is installed and available on your PATH.",
      candidate.display()
    ))
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  #[test]
  fn default_path_is_vyper() {
    assert_eq!(default_path(), PathBuf::from("vyper"));
  }

  #[test]
  fn ensure_installed_errors_for_missing_binary() {
    let path = PathBuf::from("/definitely/missing/vyper");
    assert!(ensure_installed(Some(path)).is_err());
  }
}
