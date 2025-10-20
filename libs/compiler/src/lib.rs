#[macro_use]
extern crate napi_derive;

mod ast;
mod compiler;
mod contract;
mod internal;

pub use ast::{
  Ast, FragmentTarget as AstFragmentTarget, SourceTarget as AstSourceTarget, State as AstState,
};
pub use compiler::{
  core::{
    SourceTarget as CompilerSourceTarget, SourceValue as CompilerSourceValue,
    State as CompilerState,
  },
  output::{
    from_standard_json, into_core_compile_output, CompilerError, JsCompileOutput,
    JsSourceArtifacts, SecondarySourceLocation, SeverityLevel, SourceLocation,
  },
  CompilationInput, Compiler,
};
pub use contract::{
  Contract as ContractOutput, ContractBytecode, ContractState, ImmutableSlot, JsContract,
  JsContractState,
};
pub use internal::config::{
  AstConfig, AstConfigOptions, CompilerConfig, CompilerConfigOptions, JsAstConfigOptions,
  JsCompilerConfigOptions, SolcConfig, SolcConfigOptions,
};
pub use internal::errors::{Error, Result};
pub use internal::path::ProjectPaths;
pub use internal::settings::{CompilerSettingsOptions, JsCompilerSettingsOptions};
