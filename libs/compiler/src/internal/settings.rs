use std::collections::BTreeMap;

use foundry_compilers::artifacts::{output_selection::OutputSelection, Settings};
use napi::bindgen_prelude::Result;
use serde::{Deserialize, Serialize};

use crate::internal::errors::map_napi_error;

/// Full compiler settings accepted by Foundry's solc wrapper.
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

pub(crate) fn merge_settings_json(base: &mut serde_json::Value, overrides: serde_json::Value) {
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
  Bmc,
  None,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerTarget {
  Assert,
  Contract,
  External,
  Public,
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
  Z3,
  Eld,
  Cvc4,
  EldStrict,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerTargetType {
  RecursiveDepth,
  BoundedLoop,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvmVersion {
  Byzantium,
  Constantinople,
  Petersburg,
  Istanbul,
  Berlin,
  London,
  Paris,
  Shanghai,
  Cancun,
  Prague,
}

#[napi(string_enum)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCheckerInvariantKind {
  Reentrancy,
  Contract,
}

pub fn merge_settings(base: &Settings, overrides: Option<&CompilerSettings>) -> Result<Settings> {
  match overrides {
    Some(settings) => {
      let merged = settings.clone().overlay(base)?;
      sanitize_settings(&merged)
    }
    None => Ok(base.clone()),
  }
}

pub fn sanitize_settings(settings: &Settings) -> Result<Settings> {
  let mut merged = settings.clone();
  if output_selection_is_effectively_empty(&merged.output_selection) {
    merged.output_selection = Settings::default().output_selection;
  }
  Ok(merged)
}

pub fn output_selection_is_effectively_empty(selection: &OutputSelection) -> bool {
  let map = selection.as_ref();
  if map.is_empty() {
    return true;
  }

  map.values().all(|contracts| {
    contracts
      .values()
      .all(|outputs| outputs.iter().all(|output| output.trim().is_empty()))
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sanitize_restores_default_output_selection() {
    let mut base = Settings::default();
    base.output_selection = OutputSelection::default();
    assert!(output_selection_is_effectively_empty(
      &base.output_selection
    ));

    let sanitised = sanitize_settings(&base).expect("sanitize");
    assert!(
      !output_selection_is_effectively_empty(&sanitised.output_selection),
      "sanitised selection should fall back to defaults"
    );
  }

  #[test]
  fn merge_preserves_base_when_no_overrides() {
    let base = Settings::default();
    let merged = merge_settings(&base, None).expect("merge");
    assert_eq!(
      serde_json::to_value(&base).unwrap(),
      serde_json::to_value(&merged).unwrap()
    );
  }

  #[test]
  fn merge_applies_overrides() {
    let base = Settings::default();
    let mut overrides = CompilerSettings::default();
    overrides.via_ir = Some(true);
    overrides.optimizer = Some(OptimizerSettings {
      enabled: Some(true),
      runs: Some(200),
      details: None,
    });
    let merged = merge_settings(&base, Some(&overrides)).expect("merge");
    assert_eq!(merged.via_ir, Some(true));
    assert_eq!(merged.optimizer.enabled, Some(true));
    assert_eq!(merged.optimizer.runs, Some(200));
  }
}
