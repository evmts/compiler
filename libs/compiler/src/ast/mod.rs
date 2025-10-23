use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown, ValueType};
use serde_json::Value;

pub mod core;
mod error;
mod instrumenter;
pub(crate) mod orchestrator;
pub(crate) mod parser;
mod stitcher;
pub(crate) mod utils;

#[cfg(test)]
mod ast_tests;

use core::{
  expose_internal_functions, expose_internal_variables, from_source, init, inject_shadow,
  inject_shadow_at_edges, source_unit, source_unit_mut, validate,
};
pub use core::{FragmentTarget, SourceTarget, State};
use utils::{from_js_value, to_js_value};

use crate::internal::config::{parse_js_ast_options, AstConfig, AstConfigOptions};
use crate::internal::errors::{napi_error, to_napi_result, Result};
use crate::internal::logging::ensure_napi_logger;

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

  pub fn inject_shadow_at_edges(
    &mut self,
    selector: &str,
    before: &[String],
    after: &[String],
    options: Option<AstConfigOptions>,
  ) -> Result<&mut Self> {
    inject_shadow_at_edges(&mut self.state, selector, before, after, options.as_ref())?;
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

  pub fn ast(&self) -> Result<&Value> {
    source_unit(&self.state).ok_or_else(|| {
      crate::internal::errors::Error::new("Ast has no target unit. Call from_source first.")
    })
  }

  pub fn ast_mut(&mut self) -> Result<&mut Value> {
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
  pub(crate) fn from_ast(ast: Ast) -> Self {
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
    let level = config_options
      .as_ref()
      .and_then(|opts| opts.logging_level)
      .unwrap_or_default();
    ensure_napi_logger(&env, level)?;
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

  /// Inject statements at the beginning of a function body and before every return without changing the ABI.
  /// The `selector` is the name of a function or its full signature, which might be useful if the function is overloaded.
  #[napi(
    ts_args_type = "selector: string, options: { before?: string | string[], after?: string | string[] } & AstConfigOptions",
    ts_return_type = "this"
  )]
  pub fn inject_shadow_at_edges(
    &mut self,
    env: Env,
    selector: String,
    options: Option<JsUnknown>,
  ) -> napi::Result<JsAst> {
    let (before, after, overrides) = parse_edges_options(&env, options)?;
    if before.is_empty() && after.is_empty() {
      return Err(napi_error(
        "injectShadowAtEdges requires a `before` and/or `after` snippet.",
      ));
    }
    to_napi_result(
      self
        .inner
        .inject_shadow_at_edges(&selector, &before, &after, overrides),
    )?;
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
  #[napi(ts_return_type = "import('./solc-ast').SourceUnit")]
  pub fn ast(&self, env: Env) -> napi::Result<JsUnknown> {
    let ast = self
      .inner
      .ast()
      .map_err(|err| napi_error(err.to_string()))?;
    to_js_value(&env, ast)
  }
}

fn parse_source_target(env: &Env, target: Either<String, JsObject>) -> napi::Result<SourceTarget> {
  match target {
    Either::A(source) => Ok(SourceTarget::Text(source)),
    Either::B(object) => Ok(SourceTarget::Ast(from_js_value(
      env,
      object.into_unknown(),
    )?)),
  }
}

fn parse_fragment_input(
  env: &Env,
  fragment: Either<String, JsObject>,
) -> napi::Result<FragmentTarget> {
  match fragment {
    Either::A(source) => Ok(FragmentTarget::Text(source)),
    Either::B(object) => Ok(FragmentTarget::Ast(from_js_value(
      env,
      object.into_unknown(),
    )?)),
  }
}

fn parse_edges_options(
  env: &Env,
  options: Option<JsUnknown>,
) -> napi::Result<(Vec<String>, Vec<String>, Option<AstConfigOptions>)> {
  let Some(value) = options else {
    return Ok((Vec::new(), Vec::new(), None));
  };

  let object = value.coerce_to_object()?;

  let before = extract_statement_list(&object, "before")?;
  let after = extract_statement_list(&object, "after")?;

  let overrides = parse_js_ast_options(env, Some(object.into_unknown()))?
    .as_ref()
    .map(|opts| AstConfigOptions::try_from(opts))
    .transpose()?;

  Ok((before, after, overrides))
}

fn extract_statement_list(object: &JsObject, property: &str) -> napi::Result<Vec<String>> {
  if !object.has_named_property(property)? {
    return Ok(Vec::new());
  }
  let value = object.get_named_property::<JsUnknown>(property)?;
  match value.get_type()? {
    ValueType::Undefined | ValueType::Null => Ok(Vec::new()),
    ValueType::String => {
      let js_string = value.coerce_to_string()?;
      let utf8 = js_string.into_utf8()?;
      Ok(vec![utf8.into_owned()?.trim().to_string()])
    }
    ValueType::Object => {
      if !value.is_array()? {
        return Err(napi_error(format!(
          "`{}` must be a string or an array of strings.",
          property
        )));
      }
      let array_object = value.coerce_to_object()?;
      let length = array_object.get_array_length_unchecked()?;
      let mut items = Vec::with_capacity(length as usize);
      for idx in 0..length {
        let element = array_object.get_element::<JsUnknown>(idx)?;
        if matches!(element.get_type()?, ValueType::Undefined | ValueType::Null) {
          continue;
        }
        if element.get_type()? != ValueType::String {
          return Err(napi_error(format!(
            "`{}` array entries must be strings.",
            property
          )));
        }
        let js_string = element.coerce_to_string()?;
        let utf8 = js_string.into_utf8()?;
        let value = utf8.into_owned()?.trim().to_string();
        if !value.is_empty() {
          items.push(value);
        }
      }
      Ok(items)
    }
    _ => Err(napi_error(format!(
      "`{}` must be provided as a string or array of strings.",
      property
    ))),
  }
}
