use foundry_compilers::error::SolcError;
use napi::bindgen_prelude::*;

#[derive(Debug)]
pub enum InstrumentError {
  ParseFailed(String),
  AnalysisFailed(String),
  NoNodesFound,
  InvalidContractStructure(String),
  JsonError(String),
  CompilerError(String),
}

impl std::fmt::Display for InstrumentError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ParseFailed(msg) => write!(f, "Parse failed: {}", msg),
      Self::AnalysisFailed(msg) => write!(f, "Analysis failed: {}", msg),
      Self::NoNodesFound => write!(f, "No nodes found in AST"),
      Self::InvalidContractStructure(msg) => write!(f, "Invalid contract structure: {}", msg),
      Self::JsonError(msg) => write!(f, "JSON error: {}", msg),
      Self::CompilerError(msg) => write!(f, "Compiler error: {}", msg),
    }
  }
}

impl std::error::Error for InstrumentError {}

impl From<SolcError> for InstrumentError {
  fn from(err: SolcError) -> Self {
    Self::CompilerError(err.to_string())
  }
}

impl From<serde_json::Error> for InstrumentError {
  fn from(err: serde_json::Error) -> Self {
    Self::JsonError(err.to_string())
  }
}

impl From<InstrumentError> for Error {
  fn from(err: InstrumentError) -> Self {
    Error::new(Status::GenericFailure, err.to_string())
  }
}
