use foundry_compilers::artifacts::{CompilerOutput, Contract, Error};
use foundry_compilers::solc::SolcCompiler;
use foundry_compilers::{Artifact, ProjectCompileOutput};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CoreCompilerError {
  pub message: String,
  pub severity: String,
  pub source_location: Option<CoreSourceLocation>,
}

#[derive(Debug, Clone)]
pub struct CoreSourceLocation {
  pub file: String,
  pub start: i32,
  pub end: i32,
}

#[derive(Debug, Clone)]
pub struct CoreContractArtifact {
  pub contract_name: String,
  pub abi: Option<Value>,
  pub bytecode: Option<Vec<u8>>,
  pub deployed_bytecode: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct CoreCompileOutput {
  pub artifacts: Vec<CoreContractArtifact>,
  pub errors: Vec<CoreCompilerError>,
  pub has_compiler_errors: bool,
}

pub fn into_core_compile_output(output: ProjectCompileOutput<SolcCompiler>) -> CoreCompileOutput {
  let has_compiler_errors = output.has_compiler_errors();
  let artifacts = output
    .artifacts()
    .map(|(name, artifact)| project_contract(&name, artifact))
    .collect();
  let errors = output
    .output()
    .errors
    .iter()
    .map(to_compiler_error)
    .collect();

  CoreCompileOutput {
    artifacts,
    has_compiler_errors,
    errors,
  }
}

pub fn from_standard_json(output: CompilerOutput) -> CoreCompileOutput {
  let has_compiler_errors = output.has_error();
  let CompilerOutput {
    errors, contracts, ..
  } = output;
  let artifacts = contracts
    .into_values()
    .flat_map(|set| set.into_iter())
    .map(|(name, contract)| standard_contract(name, contract))
    .collect();
  let errors = errors.iter().map(to_compiler_error).collect();

  CoreCompileOutput {
    artifacts,
    has_compiler_errors,
    errors,
  }
}

fn project_contract(name: &str, artifact: &impl Artifact) -> CoreContractArtifact {
  let bytecode_cow = artifact.get_contract_bytecode();
  let abi = bytecode_cow
    .abi
    .as_ref()
    .and_then(|abi| serde_json::to_value(&**abi).ok());
  let bytecode = bytecode_cow
    .bytecode
    .as_ref()
    .and_then(|bytecode| bytecode.object.as_bytes())
    .map(|bytes| bytes.to_vec());
  let deployed_bytecode = bytecode_cow
    .deployed_bytecode
    .as_ref()
    .and_then(|bytecode| bytecode.bytecode.as_ref())
    .and_then(|bytecode| bytecode.object.as_bytes())
    .map(|bytes| bytes.to_vec());

  CoreContractArtifact {
    contract_name: name.to_string(),
    abi,
    bytecode,
    deployed_bytecode,
  }
}

fn standard_contract(name: String, contract: Contract) -> CoreContractArtifact {
  let abi = contract
    .abi
    .as_ref()
    .and_then(|abi| serde_json::to_value(abi).ok());
  let bytecode = contract
    .evm
    .as_ref()
    .and_then(|evm| evm.bytecode.as_ref())
    .and_then(|bytecode| bytecode.object.as_bytes())
    .map(|bytes| bytes.to_vec());
  let deployed_bytecode = contract
    .evm
    .as_ref()
    .and_then(|evm| evm.deployed_bytecode.as_ref())
    .and_then(|bytecode| bytecode.bytes())
    .map(|bytes| bytes.to_vec());

  CoreContractArtifact {
    contract_name: name,
    abi,
    bytecode,
    deployed_bytecode,
  }
}

fn to_compiler_error(error: &Error) -> CoreCompilerError {
  CoreCompilerError {
    message: error.message.clone(),
    severity: format!("{:?}", error.severity),
    source_location: error
      .source_location
      .as_ref()
      .map(|loc| CoreSourceLocation {
        file: loc.file.clone(),
        start: loc.start,
        end: loc.end,
      }),
  }
}
