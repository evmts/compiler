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
pub struct ContractArtifact {
  pub contract_name: String,
  pub abi: Option<String>,
  pub bytecode: Option<String>,
  pub deployed_bytecode: Option<String>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileOutput {
  pub artifacts: Vec<ContractArtifact>,
  pub errors: Vec<CompilerError>,
  pub has_compiler_errors: bool,
}
