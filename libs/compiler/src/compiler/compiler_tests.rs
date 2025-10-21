use crate::Compiler;

#[cfg(test)]
mod tests {
  use crate::compiler::{SourceTarget, SourceValue};
  use crate::Compiler;
  use std::collections::BTreeMap;
  use std::path::{Path, PathBuf};
  use tempfile::tempdir;

  const SAMPLE_SOURCE: &str = r#"pragma solidity ^0.8.13;

contract Sample {
  function greet() public pure returns (string memory) {
    return "hello";
  }
}
"#;

  fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("test/fixtures")
      .join(path)
  }

  #[test]
  fn compile_inline_source_produces_artifact() {
    let compiler = Compiler::new(None).expect("compiler");
    let output = compiler
      .compile_source(SourceTarget::Text(SAMPLE_SOURCE.into()), None)
      .expect("compile inline");
    assert!(!output.artifacts.is_empty());
    assert!(output
      .artifact
      .as_ref()
      .and_then(|entry| entry.contracts.get("Sample"))
      .is_some());
  }

  #[test]
  fn compile_sources_accepts_btreemap() {
    let compiler = Compiler::new(None).expect("compiler");
    let mut sources = BTreeMap::new();
    sources.insert(
      "Sample.sol".to_string(),
      SourceValue::Text("contract Sample { function id() public {} }".into()),
    );
    let output = compiler
      .compile_sources(sources, None)
      .expect("compile sources");
    assert!(!output.artifacts.is_empty());
  }

  #[test]
  fn compile_files_reads_from_disk() {
    let compiler = Compiler::new(None).expect("compiler");
    let path = fixture("contracts/InlineExample.sol");
    let output = compiler
      .compile_files(vec![path], None)
      .expect("compile file");
    assert!(!output.artifacts.is_empty());
  }

  #[test]
  fn get_paths_returns_synthetic_layout() {
    let compiler = Compiler::new(None).expect("compiler");
    let paths = compiler.get_paths().expect("paths");
    assert!(Path::new(&paths.cache).parent().unwrap().exists());
    assert!(paths.virtual_sources.is_some());
  }

  #[test]
  fn from_root_uses_explicit_directory() {
    let temp = tempdir().expect("temp dir");
    let compiler = Compiler::from_root(temp.path(), None).expect("compiler");
    let paths = compiler.get_paths().expect("paths");
    let expected = temp.path().canonicalize().expect("canonical root");
    let actual = Path::new(&paths.root)
      .canonicalize()
      .expect("canonical paths root");
    assert_eq!(actual, expected);
  }

  #[test]
  fn from_hardhat_root_compiles_project() {
    let root = fixture("hardhat-project");
    let compiler = Compiler::from_hardhat_root(&root, None).expect("compiler");
    let output = compiler.compile_project(None).expect("compile project");
    assert!(!output.artifacts.is_empty());
  }

  #[test]
  fn from_foundry_root_compiles_project() {
    let root = fixture("foundry-project");
    let compiler = Compiler::from_foundry_root(&root, None).expect("compiler");
    let output = compiler.compile_project(None).expect("compile project");
    assert!(!output.artifacts.is_empty());
  }
}

#[test]
fn inline_sources_populate_artifacts_in_synthetic_context() {
  let temp_dir = tempfile::tempdir().expect("tempdir");
  let compiler = Compiler::from_root(temp_dir.path(), None).expect("compiler");

  let output = compiler
    .compile_source(
      crate::compiler::SourceTarget::Text(
        "pragma solidity ^0.8.0; contract InlineExample { function f() external {} }".into(),
      ),
      None,
    )
    .expect("compile");

  assert!(
    !output.artifacts.is_empty() || output.artifact.is_some(),
    "expected artifacts, got none"
  );
}

#[test]
fn sources_populate_artifacts_in_synthetic_context() {
  let temp_dir = tempfile::tempdir().expect("tempdir");
  let compiler = Compiler::from_root(temp_dir.path(), None).expect("compiler");

  let mut sources = std::collections::BTreeMap::new();
  sources.insert(
    "Sample.sol".to_string(),
    crate::compiler::SourceValue::Text("contract Sample { function id() public {} }".into()),
  );
  let output = compiler.compile_sources(sources, None).expect("compile");
  assert!(!output.artifacts.is_empty());
  assert!(output.artifact.is_some());
  assert!(output
    .artifact
    .as_ref()
    .unwrap()
    .contracts
    .contains_key("Sample"));
}
