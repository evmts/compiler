use foundry_compilers::solc::SolcCompiler;
use foundry_compilers::{Artifact, ProjectCompileOutput};

use crate::types::{CompileOutput, CompilerError, ContractArtifact, SourceLocation};

pub(super) fn into_compile_output(output: ProjectCompileOutput<SolcCompiler>) -> CompileOutput {
  let has_compiler_errors = output.has_compiler_errors();
  let mut artifacts = Vec::new();
  for (name, artifact) in output.artifacts() {
    let abi = artifact
      .abi
      .as_ref()
      .and_then(|abi| serde_json::to_string(abi).ok());
    artifacts.push(ContractArtifact {
      contract_name: name.clone(),
      abi,
      bytecode: artifact
        .get_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref().as_ref()))),
      deployed_bytecode: artifact
        .get_deployed_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref().as_ref()))),
    });
  }
  let errors = output
    .output()
    .errors
    .iter()
    .map(to_compiler_error)
    .collect();

  CompileOutput {
    artifacts,
    has_compiler_errors,
    errors,
  }
}

fn to_compiler_error(error: &foundry_compilers::artifacts::Error) -> CompilerError {
  CompilerError {
    message: error.message.clone(),
    severity: format!("{:?}", error.severity),
    source_location: error.source_location.as_ref().map(|loc| SourceLocation {
      file: loc.file.clone(),
      start: loc.start,
      end: loc.end,
    }),
  }
}
