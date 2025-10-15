use foundry_compilers::{
  solc::{SolcCompiler, SolcLanguage},
  Artifact, Project, ProjectBuilder, ProjectPathsConfig,
};
use napi::bindgen_prelude::*;
use std::path::PathBuf;

use crate::types::{CompileOutput, CompilerError, ContractArtifact, SourceLocation};

#[napi]
pub struct SolidityProject {
  pub(crate) project: Project<SolcCompiler>,
}

#[napi]
impl SolidityProject {
  /// Create a new project from a root path using Hardhat layout
  #[napi(factory)]
  pub fn from_hardhat_root(root_path: String) -> Result<Self> {
    let root = PathBuf::from(&root_path);
    let paths: ProjectPathsConfig<SolcLanguage> =
      ProjectPathsConfig::hardhat(&root).map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to create hardhat paths: {}", e),
        )
      })?;

    let project = ProjectBuilder::default()
      .paths(paths)
      .build(SolcCompiler::default())
      .map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to build project: {}", e),
        )
      })?;

    Ok(SolidityProject { project })
  }

  /// Create a new project from a root path using Dapptools layout
  #[napi(factory)]
  pub fn from_dapptools_root(root_path: String) -> Result<Self> {
    let root = PathBuf::from(&root_path);
    let paths: ProjectPathsConfig<SolcLanguage> =
      ProjectPathsConfig::dapptools(&root).map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to create dapptools paths: {}", e),
        )
      })?;

    let project = ProjectBuilder::default()
      .paths(paths)
      .build(SolcCompiler::default())
      .map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to build project: {}", e),
        )
      })?;

    Ok(SolidityProject { project })
  }

  /// Compile all contracts in the project
  #[napi]
  pub fn compile(&self) -> Result<CompileOutput> {
    let output = self
      .project
      .compile()
      .map_err(|e| Error::new(Status::GenericFailure, format!("Compilation failed: {}", e)))?;

    let mut artifacts = Vec::new();
    for (name, artifact) in output.artifacts() {
      let abi_json = artifact
        .abi
        .as_ref()
        .map(|abi| serde_json::to_string(abi).unwrap_or_default());

      let bytecode = artifact
        .get_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

      let deployed_bytecode = artifact
        .get_deployed_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

      artifacts.push(ContractArtifact {
        contract_name: name.clone(),
        abi: abi_json,
        bytecode,
        deployed_bytecode,
      });
    }

    let errors: Vec<CompilerError> = output
      .output()
      .errors
      .iter()
      .map(|err| CompilerError {
        message: err.message.clone(),
        severity: format!("{:?}", err.severity),
        source_location: err.source_location.as_ref().map(|loc| SourceLocation {
          file: loc.file.clone(),
          start: loc.start,
          end: loc.end,
        }),
      })
      .collect();

    Ok(CompileOutput {
      artifacts,
      has_compiler_errors: output.has_compiler_errors(),
      errors,
    })
  }

  /// Compile a single file
  #[napi]
  pub fn compile_file(&self, file_path: String) -> Result<CompileOutput> {
    let path = PathBuf::from(file_path);
    let output = self
      .project
      .compile_file(&path)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Compilation failed: {}", e)))?;

    let mut artifacts = Vec::new();
    for (name, artifact) in output.artifacts() {
      let abi_json = artifact
        .abi
        .as_ref()
        .map(|abi| serde_json::to_string(abi).unwrap_or_default());

      let bytecode = artifact
        .get_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

      let deployed_bytecode = artifact
        .get_deployed_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

      artifacts.push(ContractArtifact {
        contract_name: name.clone(),
        abi: abi_json,
        bytecode,
        deployed_bytecode,
      });
    }

    let errors: Vec<CompilerError> = output
      .output()
      .errors
      .iter()
      .map(|err| CompilerError {
        message: err.message.clone(),
        severity: format!("{:?}", err.severity),
        source_location: err.source_location.as_ref().map(|loc| SourceLocation {
          file: loc.file.clone(),
          start: loc.start,
          end: loc.end,
        }),
      })
      .collect();

    Ok(CompileOutput {
      artifacts,
      has_compiler_errors: output.has_compiler_errors(),
      errors,
    })
  }

  /// Compile multiple files
  #[napi]
  pub fn compile_files(&self, file_paths: Vec<String>) -> Result<CompileOutput> {
    let paths: Vec<PathBuf> = file_paths.iter().map(PathBuf::from).collect();
    let output = self
      .project
      .compile_files(paths)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Compilation failed: {}", e)))?;

    let mut artifacts = Vec::new();
    for (name, artifact) in output.artifacts() {
      let abi_json = artifact
        .abi
        .as_ref()
        .map(|abi| serde_json::to_string(abi).unwrap_or_default());

      let bytecode = artifact
        .get_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

      let deployed_bytecode = artifact
        .get_deployed_bytecode_bytes()
        .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

      artifacts.push(ContractArtifact {
        contract_name: name.clone(),
        abi: abi_json,
        bytecode,
        deployed_bytecode,
      });
    }

    let errors: Vec<CompilerError> = output
      .output()
      .errors
      .iter()
      .map(|err| CompilerError {
        message: err.message.clone(),
        severity: format!("{:?}", err.severity),
        source_location: err.source_location.as_ref().map(|loc| SourceLocation {
          file: loc.file.clone(),
          start: loc.start,
          end: loc.end,
        }),
      })
      .collect();

    Ok(CompileOutput {
      artifacts,
      has_compiler_errors: output.has_compiler_errors(),
      errors,
    })
  }

  /// Find the path of a contract by its name
  #[napi]
  pub fn find_contract_path(&self, contract_name: String) -> Result<String> {
    let path = self
      .project
      .find_contract_path(&contract_name)
      .map_err(|e| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to find contract: {}", e),
        )
      })?;

    Ok(path.to_string_lossy().to_string())
  }

  /// Get all source files in the project
  #[napi]
  pub fn get_sources(&self) -> Result<Vec<String>> {
    let sources = self.project.sources().map_err(|e| {
      Error::new(
        Status::GenericFailure,
        format!("Failed to get sources: {}", e),
      )
    })?;

    Ok(
      sources
        .keys()
        .map(|p| p.to_string_lossy().to_string())
        .collect(),
    )
  }
}
