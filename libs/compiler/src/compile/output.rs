use foundry_compilers::artifacts::{CompilerOutput, Contract, Error};
use foundry_compilers::solc::SolcCompiler;
use foundry_compilers::{Artifact, ProjectCompileOutput};

use crate::types::{CompileOutput, CompilerError, ContractArtifact, SourceLocation};

pub(crate) fn into_compile_output(output: ProjectCompileOutput<SolcCompiler>) -> CompileOutput {
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

  CompileOutput {
    artifacts,
    has_compiler_errors,
    errors,
  }
}

pub(crate) fn from_standard_json(output: CompilerOutput) -> CompileOutput {
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

  CompileOutput {
    artifacts,
    has_compiler_errors,
    errors,
  }
}

fn project_contract(name: &str, artifact: &impl Artifact) -> ContractArtifact {
  let bytecode_cow = artifact.get_contract_bytecode();
  let abi = bytecode_cow
    .abi
    .as_ref()
    .and_then(|abi| serde_json::to_string(&**abi).ok());
  let bytecode = bytecode_cow
    .bytecode
    .as_ref()
    .and_then(|bytecode| bytecode.object.as_bytes())
    .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));
  let deployed_bytecode = bytecode_cow
    .deployed_bytecode
    .as_ref()
    .and_then(|bytecode| bytecode.bytecode.as_ref())
    .and_then(|bytecode| bytecode.object.as_bytes())
    .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

  ContractArtifact {
    contract_name: name.to_string(),
    abi,
    bytecode,
    deployed_bytecode,
  }
}

fn standard_contract(name: String, contract: Contract) -> ContractArtifact {
  let abi = contract
    .abi
    .as_ref()
    .and_then(|abi| serde_json::to_string(abi).ok());
  let bytecode = contract
    .evm
    .as_ref()
    .and_then(|evm| evm.bytecode.as_ref())
    .and_then(|bytecode| bytecode.object.as_bytes())
    .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));
  let deployed_bytecode = contract
    .evm
    .as_ref()
    .and_then(|evm| evm.deployed_bytecode.as_ref())
    .and_then(|bytecode| bytecode.bytes())
    .map(|bytes| format!("0x{}", hex::encode(bytes.as_ref())));

  ContractArtifact {
    contract_name: name,
    abi,
    bytecode,
    deployed_bytecode,
  }
}

fn to_compiler_error(error: &Error) -> CompilerError {
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
