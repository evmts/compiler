use serde_json::Value;

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
