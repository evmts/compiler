use foundry_compilers::artifacts::Settings;
use napi::bindgen_prelude::*;
use napi::{Env, JsObject, JsUnknown, NapiRaw, ValueType};
use semver::Version;

use super::{errors::napi_error, settings::CompilerSettings, solc};

pub(crate) trait SolcUserOptions {
  fn solc_version(&self) -> Option<&str>;
  fn settings(&self) -> Option<&CompilerSettings>;
}

macro_rules! define_options_struct {
  ($(#[$meta:meta])* $name:ident) => {
    $(#[$meta])*
    #[napi(object)]
    #[derive(Clone, Default)]
    pub struct $name {
      #[napi(ts_type = "string | undefined")]
      pub solc_version: Option<String>,
      #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
      pub settings: Option<CompilerSettings>,
    }

    impl SolcUserOptions for $name {
      fn solc_version(&self) -> Option<&str> {
        self.solc_version.as_deref()
      }

      fn settings(&self) -> Option<&CompilerSettings> {
        self.settings.as_ref()
      }
    }
  };
}

define_options_struct!(
  /// Shared solc configuration accepted by compiler entry points.
  CompilerOptions
);

#[napi(object)]
#[derive(Clone, Default)]
pub struct AstOptions {
  #[napi(ts_type = "string | undefined")]
  pub solc_version: Option<String>,
  #[napi(ts_type = "import('./index').CompilerSettings | undefined")]
  pub settings: Option<CompilerSettings>,
  #[napi(ts_type = "string | undefined")]
  pub instrumented_contract: Option<String>,
}

impl SolcUserOptions for AstOptions {
  fn solc_version(&self) -> Option<&str> {
    self.solc_version.as_deref()
  }

  fn settings(&self) -> Option<&CompilerSettings> {
    self.settings.as_ref()
  }
}

#[derive(Clone)]
pub(crate) struct SolcConfig {
  pub version: Version,
  pub settings: Settings,
}

impl SolcConfig {
  pub fn new<O: SolcUserOptions>(
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let default_version = solc::default_version()?;
    Self::with_defaults(&default_version, default_settings, overrides)
  }

  pub fn with_defaults<O: SolcUserOptions>(
    default_version: &Version,
    default_settings: &Settings,
    overrides: Option<&O>,
  ) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .map(solc::parse_version)
      .transpose()?
      .unwrap_or_else(|| default_version.clone());

    let settings = resolve_settings(default_settings, overrides.and_then(|opts| opts.settings()))?;

    Ok(SolcConfig { version, settings })
  }

  pub fn merge<O: SolcUserOptions>(&self, overrides: Option<&O>) -> Result<Self> {
    let version = overrides
      .and_then(|opts| opts.solc_version())
      .map(solc::parse_version)
      .transpose()?
      .unwrap_or_else(|| self.version.clone());

    let settings = resolve_settings(&self.settings, overrides.and_then(|opts| opts.settings()))?;

    Ok(SolcConfig { version, settings })
  }
}

pub(crate) fn default_compiler_settings() -> Settings {
  Settings::default()
}

fn resolve_settings(base: &Settings, overrides: Option<&CompilerSettings>) -> Result<Settings> {
  match overrides {
    Some(settings) => {
      let mut merged = settings.clone().overlay(base)?;
      if merged.output_selection.as_ref().is_empty() {
        merged.output_selection = Settings::default().output_selection;
      }
      Ok(merged)
    }
    None => Ok(base.clone()),
  }
}

pub(crate) fn parse_compiler_options(
  env: &Env,
  value: Option<JsUnknown>,
) -> Result<Option<CompilerOptions>> {
  parse_options(value)?
    .map(|unknown| unsafe { CompilerOptions::from_napi_value(env.raw(), unknown.raw()) })
    .transpose()
}

pub(crate) fn parse_ast_options(env: &Env, value: Option<JsUnknown>) -> Result<Option<AstOptions>> {
  parse_options(value)?
    .map(|unknown| unsafe { AstOptions::from_napi_value(env.raw(), unknown.raw()) })
    .transpose()
}

impl AstOptions {
  pub fn instrumented_contract(&self) -> Option<&str> {
    self.instrumented_contract.as_deref()
  }
}

fn parse_options(value: Option<JsUnknown>) -> Result<Option<JsUnknown>> {
  let Some(value) = value else {
    return Ok(None);
  };

  match value.get_type()? {
    ValueType::Undefined | ValueType::Null => Ok(None),
    ValueType::Object => {
      let object: JsObject = value.coerce_to_object()?;

      if object.has_named_property("solcVersion")? {
        let solc_value = object.get_named_property::<JsUnknown>("solcVersion")?;
        match solc_value.get_type()? {
          ValueType::Undefined | ValueType::Null | ValueType::String => {}
          _ => {
            return Err(napi_error("solcVersion must be a string when provided."));
          }
        }
      }

      if object.has_named_property("settings")? {
        let settings_value = object.get_named_property::<JsUnknown>("settings")?;
        match settings_value.get_type()? {
          ValueType::Undefined | ValueType::Null | ValueType::Object => {}
          _ => {
            return Err(napi_error(
              "Solc settings override must be provided as an object value.",
            ));
          }
        }
      }

      if object.has_named_property("instrumentedContract")? {
        let contract_value = object.get_named_property::<JsUnknown>("instrumentedContract")?;
        match contract_value.get_type()? {
          ValueType::Undefined | ValueType::Null | ValueType::String => {}
          _ => {
            return Err(napi_error(
              "instrumentedContract must be a string when provided.",
            ));
          }
        }
      }

      Ok(Some(object.into_unknown()))
    }
    _ => Err(napi_error("Options must be provided as an object.")),
  }
}
