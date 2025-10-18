use foundry_compilers::artifacts::{
  ast::{Node, NodeType},
  CompilerOutput, Contract, Error, SourceFile,
};
use foundry_compilers::solc::SolcCompiler;
use foundry_compilers::{Artifact, ProjectCompileOutput};
use serde_json::Value;
use std::collections::BTreeSet;

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompilerError {
  pub message: String,
  pub severity: String,
  pub source_location: Option<SourceLocation>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct SourceLocation {
  pub file: String,
  pub start: i32,
  pub end: i32,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ContractBytecode {
  pub hex: Option<String>,
  #[napi(ts_type = "Uint8Array | undefined")]
  pub bytes: Option<Vec<u8>>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ContractArtifact {
  pub contract_name: String,
  #[napi(ts_type = "unknown | undefined")]
  pub abi: Option<Value>,
  pub abi_json: Option<String>,
  pub bytecode: Option<ContractBytecode>,
  pub deployed_bytecode: Option<ContractBytecode>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileOutput {
  pub artifacts: Vec<ContractArtifact>,
  pub errors: Vec<CompilerError>,
  pub has_compiler_errors: bool,
}

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
    errors,
    contracts,
    sources,
    ..
  } = output;
  let mut artifacts: Vec<CoreContractArtifact> = contracts
    .into_values()
    .flat_map(|set| set.into_iter())
    .map(|(name, contract)| standard_contract(name, contract))
    .collect();
  // TODO: remove that, just a stub until we actually return the sources
  if artifacts.is_empty() {
    artifacts = contract_stubs_from_ast(&sources)
      .into_iter()
      .map(|name| CoreContractArtifact {
        contract_name: name,
        abi: None,
        bytecode: None,
        deployed_bytecode: None,
      })
      .collect();
  }
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

fn contract_stubs_from_ast(
  sources: &std::collections::BTreeMap<std::path::PathBuf, SourceFile>,
) -> Vec<String> {
  let mut names = BTreeSet::new();
  for source in sources.values() {
    if let Some(ast) = &source.ast {
      collect_contract_names(&ast.nodes, &mut names);
    }
  }
  names.into_iter().collect()
}

fn collect_contract_names(nodes: &[Node], acc: &mut BTreeSet<String>) {
  for node in nodes {
    if matches!(node.node_type, NodeType::ContractDefinition) {
      if let Some(name) = node.attribute::<String>("name") {
        if !name.is_empty() {
          acc.insert(name);
        }
      }
    }
    if let Some(body) = node.body.as_deref() {
      collect_contract_names(std::slice::from_ref(body), acc);
    }
    if !node.nodes.is_empty() {
      collect_contract_names(&node.nodes, acc);
    }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn from_standard_json_converts_artifacts_and_errors() {
    let json = r#"{
      "contracts": {
        "Test.sol": {
          "Test": {
            "abi": [],
            "evm": {
              "bytecode": { "object": "0x6000" },
              "deployedBytecode": { "bytecode": { "object": "0x6001" } }
            }
          }
        }
      },
      "errors": [
        {
          "component": "general",
          "formattedMessage": "Error: failure",
          "message": "failure",
          "severity": "error",
          "type": "ParserError",
          "sourceLocation": { "file": "Test.sol", "start": 0, "end": 10 }
        }
      ],
      "sources": {},
      "version": "0.8.30"
    }"#;

    let output: CompilerOutput = serde_json::from_str(json).expect("parse compiler output");
    let core = from_standard_json(output);

    assert!(core.has_compiler_errors);
    assert_eq!(core.artifacts.len(), 1);
    let artifact = &core.artifacts[0];
    assert_eq!(artifact.contract_name, "Test");
    assert_eq!(artifact.abi.as_ref().unwrap(), &serde_json::json!([]));
    if let Some(bytecode) = &artifact.bytecode {
      assert!(!bytecode.is_empty());
    }
    if let Some(deployed) = &artifact.deployed_bytecode {
      assert!(!deployed.is_empty());
    }

    assert_eq!(core.errors.len(), 1);
    let error = &core.errors[0];
    assert_eq!(error.message, "failure");
    assert_eq!(error.severity, "Error");
    let location = error.source_location.as_ref().expect("location");
    assert_eq!(location.file, "Test.sol");
    assert_eq!(location.start, 0);
    assert_eq!(location.end, 10);
  }

  #[test]
  fn from_standard_json_falls_back_to_ast_contracts() {
    use foundry_compilers::artifacts::ast::{Ast, LowFidelitySourceLocation, Node, NodeType};
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    let ast = Ast {
      absolute_path: "InlineExample.sol".to_string(),
      id: 1,
      exported_symbols: Default::default(),
      node_type: NodeType::SourceUnit,
      src: "0:0:0".parse::<LowFidelitySourceLocation>().expect("src"),
      nodes: vec![Node {
        id: Some(2),
        node_type: NodeType::ContractDefinition,
        src: "0:0:0".parse::<LowFidelitySourceLocation>().expect("src"),
        nodes: vec![],
        body: None,
        other: BTreeMap::from([("name".to_string(), json!("InlineExample"))]),
      }],
      other: Default::default(),
    };

    let source_file = SourceFile {
      id: 1,
      ast: Some(ast),
    };
    let mut sources = BTreeMap::new();
    sources.insert(PathBuf::from("InlineExample.sol"), source_file);

    let mut output: CompilerOutput = Default::default();
    output.sources = sources;

    let core = from_standard_json(output);
    assert!(!core.has_compiler_errors);
    assert_eq!(core.artifacts.len(), 1);
    assert_eq!(core.artifacts[0].contract_name, "InlineExample");
    assert!(core.artifacts[0].abi.is_none());
    assert!(core.artifacts[0].bytecode.is_none());
    assert!(core.artifacts[0].deployed_bytecode.is_none());
  }

  #[test]
  fn to_compiler_error_rewrites_fields() {
    let json = r#"{
      "component": "general",
      "formattedMessage": "Warning: detail",
      "message": "detail",
      "severity": "warning",
      "type": "Warning",
      "sourceLocation": { "file": "Lib.sol", "start": 1, "end": 2 }
    }"#;
    let error: Error = serde_json::from_str(json).expect("parse error");
    let converted = super::to_compiler_error(&error);

    assert_eq!(converted.message, "detail");
    assert_eq!(converted.severity, "Warning");
    let location = converted.source_location.expect("location");
    assert_eq!(location.file, "Lib.sol");
    assert_eq!(location.start, 1);
    assert_eq!(location.end, 2);
  }
}
