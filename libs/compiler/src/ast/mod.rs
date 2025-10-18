use foundry_compilers::artifacts::ast::SourceUnit;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown};

pub mod core;
mod error;
pub(crate) mod orchestrator;
pub(crate) mod parser;
mod stitcher;
pub(crate) mod utils;

#[cfg(test)]
mod ast_tests;

use core::{
  expose_internal_functions, expose_internal_variables, from_source, init, inject_shadow,
  source_unit, source_unit_mut, validate,
};
pub use core::{FragmentTarget, SourceTarget, State};
use utils::{from_js_value, sanitize_ast_value, to_js_value};

use crate::internal::config::{parse_js_ast_options, AstConfig, AstConfigOptions};
use crate::internal::errors::{map_napi_error, napi_error, to_napi_result, Result};

/// Pure Rust façade around the AST core functions.
#[derive(Clone)]
pub struct Ast {
  state: State,
}

impl Ast {
  pub fn new(options: Option<AstConfigOptions>) -> Result<Self> {
    init(options).map(|state| Self { state })
  }

  pub fn from_source(
    &mut self,
    target: SourceTarget,
    options: Option<AstConfigOptions>,
  ) -> Result<&mut Self> {
    from_source(&mut self.state, target, options.as_ref())?;
    Ok(self)
  }

  pub fn inject_shadow(
    &mut self,
    fragment: FragmentTarget,
    options: Option<AstConfigOptions>,
  ) -> Result<&mut Self> {
    inject_shadow(&mut self.state, fragment, options.as_ref())?;
    Ok(self)
  }

  pub fn expose_internal_variables(
    &mut self,
    options: Option<AstConfigOptions>,
  ) -> Result<&mut Self> {
    expose_internal_variables(&mut self.state, options.as_ref())?;
    Ok(self)
  }

  pub fn expose_internal_functions(
    &mut self,
    options: Option<AstConfigOptions>,
  ) -> Result<&mut Self> {
    expose_internal_functions(&mut self.state, options.as_ref())?;
    Ok(self)
  }

  /// Compile the current AST to ensure it represents a valid contract and refresh its references.
  /// This is optional—`ast()` already returns the parsed tree you can work with directly.
  pub fn validate(&mut self, options: Option<AstConfigOptions>) -> Result<&mut Self> {
    validate(&mut self.state, options.as_ref())?;
    Ok(self)
  }

  pub fn ast(&self) -> Result<&SourceUnit> {
    source_unit(&self.state).ok_or_else(|| {
      crate::internal::errors::Error::new("Ast has no target unit. Call from_source first.")
    })
  }

  pub fn ast_mut(&mut self) -> Result<&mut SourceUnit> {
    source_unit_mut(&mut self.state).ok_or_else(|| {
      crate::internal::errors::Error::new("Ast has no target unit. Call from_source first.")
    })
  }

  pub fn config(&self) -> &AstConfig {
    &self.state.config
  }

  pub fn config_mut(&mut self) -> &mut AstConfig {
    &mut self.state.config
  }

  pub fn into_state(self) -> State {
    self.state
  }
}

/// High-level helper for manipulating Solidity ASTs prior to recompilation.
#[napi(js_name = "Ast")]
#[derive(Clone)]
pub struct JsAst {
  inner: Ast,
}

impl JsAst {
  fn from_ast(ast: Ast) -> Self {
    Self { inner: ast }
  }
}

#[napi]
impl JsAst {
  /// Create a new AST helper. Providing `instrumentedContract` establishes the instrumented
  /// contract targeted by subsequent operations.
  #[napi(constructor, ts_args_type = "options?: AstConfigOptions | undefined")]
  pub fn new(env: Env, options: Option<JsUnknown>) -> napi::Result<Self> {
    let parsed = parse_js_ast_options(&env, options)?;
    let config_options = parsed
      .as_ref()
      .map(|opts| AstConfigOptions::try_from(opts))
      .transpose()?;
    let ast = to_napi_result(Ast::new(config_options))?;
    Ok(Self::from_ast(ast))
  }

  /// Parse Solidity source into an AST using the configured solc version. When no
  /// `instrumentedContract` is provided, later operations apply to all contracts in the file.
  #[napi(
    ts_args_type = "target: string | object, options?: AstConfigOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn from_source(
    &mut self,
    env: Env,
    target: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsAst> {
    let parsed = parse_js_ast_options(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| AstConfigOptions::try_from(opts))
      .transpose()?;
    let target = parse_source_target(&env, target)?;
    to_napi_result(self.inner.from_source(target, overrides))?;
    Ok(self.clone())
  }

  /// Parse an AST fragment from source text or inject a pre-parsed AST fragment into the targeted
  /// contract.
  #[napi(
    ts_args_type = "fragment: string | object, options?: AstConfigOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn inject_shadow(
    &mut self,
    env: Env,
    fragment: Either<String, JsObject>,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsAst> {
    let parsed = parse_js_ast_options(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| AstConfigOptions::try_from(opts))
      .transpose()?;
    let fragment = parse_fragment_input(&env, fragment)?;
    to_napi_result(self.inner.inject_shadow(fragment, overrides))?;
    Ok(self.clone())
  }

  /// Promote private/internal state variables to public visibility. Omitting `instrumentedContract`
  /// applies the change to all contracts.
  #[napi(
    ts_args_type = "options?: AstConfigOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn expose_internal_variables(
    &mut self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsAst> {
    let parsed = parse_js_ast_options(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| AstConfigOptions::try_from(opts))
      .transpose()?;
    to_napi_result(self.inner.expose_internal_variables(overrides))?;
    Ok(self.clone())
  }

  /// Promote private/internal functions to public visibility. Omitting `instrumentedContract`
  /// applies the change to all contracts.
  #[napi(
    ts_args_type = "options?: AstConfigOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn expose_internal_functions(
    &mut self,
    env: Env,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsAst> {
    let parsed = parse_js_ast_options(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| AstConfigOptions::try_from(opts))
      .transpose()?;
    to_napi_result(self.inner.expose_internal_functions(overrides))?;
    Ok(self.clone())
  }

  /// Compile the current AST to ensure it represents a valid contract and refresh its references.
  /// This is optional—`ast()` already returns the parsed tree you can work with directly.
  #[napi(
    ts_args_type = "options?: AstConfigOptions | undefined",
    ts_return_type = "this"
  )]
  pub fn validate(&mut self, env: Env, options: Option<JsUnknown>) -> napi::Result<JsAst> {
    let parsed = parse_js_ast_options(&env, options)?;
    let overrides = parsed
      .as_ref()
      .map(|opts| AstConfigOptions::try_from(opts))
      .transpose()?;
    to_napi_result(self.inner.validate(overrides))?;
    Ok(self.clone())
  }

  /// Get the current instrumented AST.
  #[napi(ts_return_type = "import('./ast-types').SourceUnit")]
  pub fn ast(&self, env: Env) -> napi::Result<JsUnknown> {
    let ast = self
      .inner
      .ast()
      .map_err(|err| napi_error(err.to_string()))?;
    let mut ast_value = map_napi_error(serde_json::to_value(ast), "Failed to serialize AST value")?;
    sanitize_ast_value(&mut ast_value);
    to_js_value(&env, &ast_value)
  }
}

fn parse_source_target(env: &Env, target: Either<String, JsObject>) -> napi::Result<SourceTarget> {
  match target {
    Either::A(source) => Ok(SourceTarget::Text(source)),
    Either::B(object) => {
      let unit: SourceUnit = from_js_value(env, object.into_unknown())?;
      Ok(SourceTarget::Ast(unit))
    }
  }
}

fn parse_fragment_input(
  env: &Env,
  fragment: Either<String, JsObject>,
) -> napi::Result<FragmentTarget> {
  match fragment {
    Either::A(source) => Ok(FragmentTarget::Text(source)),
    Either::B(object) => {
      let unit: SourceUnit = from_js_value(env, object.into_unknown())?;
      Ok(FragmentTarget::Ast(unit))
    }
  }
}
