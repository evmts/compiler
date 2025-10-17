use std::collections::BTreeMap;

use foundry_compilers::artifacts::Settings;
use napi::bindgen_prelude::Result;
use serde::{Deserialize, Serialize};

use super::errors::map_napi_error;

/// Full compiler settings accepted by Foundry's solc wrapper.
///
/// This struct mirrors [`foundry_compilers::artifacts::Settings`] and the nested
/// configuration types, but it is shaped for ergonomic consumption from
/// JavaScript. All fields are optional so callers can provide partial objects;
/// omitted values fall back to solc's defaults when converted back into the
/// Foundry representation.
#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerSettings {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "'parsing' | undefined")]
  pub stop_after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "`${string}=${string}`[] | undefined")]
  pub remappings: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub optimizer: Option<OptimizerSettings>,
  #[serde(rename = "modelChecker", skip_serializing_if = "Option::is_none")]
  pub model_checker: Option<ModelCheckerSettings>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub metadata: Option<SettingsMetadata>,
  #[serde(rename = "outputSelection", skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "import('./solc-types').OutputSelection | undefined")]
  pub output_selection: Option<BTreeMap<String, BTreeMap<String, Vec<String>>>>,
  #[serde(rename = "evmVersion", skip_serializing_if = "Option::is_none")]
  pub evm_version: Option<EvmVersion>,
  #[serde(rename = "viaIR", skip_serializing_if = "Option::is_none")]
  pub via_ir: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebuggingSettings>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[napi(ts_type = "Record<string, Record<string, string>> | undefined")]
  pub libraries: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

impl CompilerSettings {
  pub(crate) fn overlay(self, base: &Settings) -> Result<Settings> {
    let mut base_value = map_napi_error(
      serde_json::to_value(base),
      "Failed to serialise base compiler settings",
    )?;
    let overrides = map_napi_error(
      serde_json::to_value(self),
      "Failed to serialise compiler settings",
    )?;

    merge_settings_json(&mut base_value, overrides);

    map_napi_error(
      serde_json::from_value(base_value),
      "Failed to parse compiler settings",
    )
  }
}

fn merge_settings_json(base: &mut serde_json::Value, overrides: serde_json::Value) {
  match (base, overrides) {
    (serde_json::Value::Object(base_map), serde_json::Value::Object(overrides_map)) => {
      for (key, value) in overrides_map {
        match base_map.get_mut(&key) {
          Some(existing) => merge_settings_json(existing, value),
          None => {
            base_map.insert(key, value);
          }
        }
      }
    }
    (target, value) => {
      *target = value;
    }
  }
}

#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerSettings {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub enabled: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub runs: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<OptimizerDetails>,
}

#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerDetails {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub peephole: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub inliner: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub jumpdest_remover: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub order_literals: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deduplicate: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cse: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub constant_optimizer: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub yul: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub yul_details: Option<YulDetails>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub simple_counter_for_loop_unchecked_increment: Option<bool>,
}

#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YulDetails {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub stack_allocation: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub optimizer_steps: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggingSettings {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub revert_strings: Option<RevertStrings>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub debug_info: Vec<String>,
}

#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsMetadata {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub use_literal_content: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bytecode_hash: Option<BytecodeHash>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cbor_metadata: Option<bool>,
}

#[napi(object)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCheckerSettings {
  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  #[napi(ts_type = "Record<string, string[]> | undefined")]
  pub contracts: BTreeMap<String, Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub engine: Option<ModelCheckerEngine>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeout: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub targets: Option<Vec<ModelCheckerTarget>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub invariants: Option<Vec<ModelCheckerInvariant>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_unproved: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub div_mod_with_slacks: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub solvers: Option<Vec<ModelCheckerSolver>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_unsupported: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub show_proved_safe: Option<bool>,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BytecodeHash {
  Ipfs,
  None,
  Bzzr1,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RevertStrings {
  Default,
  Strip,
  Debug,
  VerboseDebug,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerEngine {
  Default,
  All,
  Bmc,
  Chc,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerTarget {
  Assert,
  Underflow,
  Overflow,
  DivByZero,
  ConstantCondition,
  PopEmptyArray,
  OutOfBounds,
  Balance,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerInvariant {
  Contract,
  Reentrancy,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerSolver {
  Cvc4,
  Eld,
  Smtlib2,
  Z3,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvmVersion {
  Homestead,
  TangerineWhistle,
  SpuriousDragon,
  Byzantium,
  Constantinople,
  Petersburg,
  Istanbul,
  Berlin,
  London,
  Paris,
  Prague,
  Shanghai,
  Cancun,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn optimizer_override_retains_default_outputs() {
    let mut settings = CompilerSettings::default();
    settings.optimizer = Some(OptimizerSettings {
      enabled: Some(true),
      runs: Some(1),
      details: None,
    });

    let merged = settings.overlay(&Settings::default()).expect("settings");
    assert!(
      !merged.output_selection.as_ref().is_empty(),
      "output selection should contain default entries"
    );
  }

  #[test]
  fn overlay_preserves_existing_defaults() {
    let base = Settings::default();
    let mut overrides = CompilerSettings::default();
    overrides.optimizer = Some(OptimizerSettings {
      enabled: Some(true),
      runs: Some(1),
      details: None,
    });

    let merged = overrides.overlay(&base).expect("settings");
    assert!(
      !merged.output_selection.as_ref().is_empty(),
      "output selection should not be empty after overlay"
    );
  }
}
