use std::path::PathBuf;

use super::input::CompilationInput;
use super::output::{into_core_compile_output, CompileOutput};
use crate::internal::config::CompilerLanguage;
use crate::internal::vyper;
use crate::internal::{
  config::CompilerConfig,
  errors::{map_err_with_context, Error, Result},
  project::{
    build_project, create_synthetic_context, default_cache_dir, ProjectContext, ProjectLayout,
  },
  solc,
};
use foundry_compilers::artifacts::sources::Source as FoundrySource;
use foundry_compilers::compilers::multi::MultiCompiler;
use foundry_compilers::{Project, ProjectCompileOutput};

pub struct ProjectRunner<'a> {
  context: &'a ProjectContext,
}

impl<'a> ProjectRunner<'a> {
  pub fn new(context: &'a ProjectContext) -> Self {
    Self { context }
  }

  pub fn compile(
    &self,
    config: &CompilerConfig,
    input: &CompilationInput,
  ) -> Result<Option<CompileOutput>> {
    match input {
      CompilationInput::InlineSource { source } => {
        if matches!(self.context.layout, ProjectLayout::Synthetic) && config.cache_enabled {
          let path = self.write_virtual_source(config, source)?;
          let output = self.compile_with_project(config, "Compilation failed", |project| {
            project.compile_file(path)
          });
          output.map(|out| Some(into_core_compile_output(out)))
        } else {
          Ok(None)
        }
      }
      CompilationInput::FilePaths { paths, .. } => {
        if matches!(self.context.layout, ProjectLayout::Synthetic) {
          return Ok(None);
        }
        let normalized = self.context.normalise_paths(paths.as_slice())?;
        let output = self.compile_with_project(config, "Compilation failed", |project| {
          project.compile_files(normalized)
        });
        output.map(|out| Some(into_core_compile_output(out)))
      }
      CompilationInput::SourceMap { .. } | CompilationInput::AstUnits { .. } => Ok(None),
    }
  }

  pub fn compile_project(&self, config: &CompilerConfig) -> Result<CompileOutput> {
    let output = self.compile_with_project(config, "Project compilation failed", |project| {
      project.compile()
    });
    output.map(into_core_compile_output)
  }

  pub fn compile_contract(
    &self,
    config: &CompilerConfig,
    contract_name: &str,
  ) -> Result<CompileOutput> {
    let name = contract_name.to_owned();
    let output = self.compile_with_project(config, "Contract compilation failed", move |project| {
      let path = project.find_contract_path(&name)?;
      project.compile_file(path)
    });
    output.map(into_core_compile_output)
  }

  fn compile_with_project<F>(
    &self,
    config: &CompilerConfig,
    label: &str,
    compile_fn: F,
  ) -> Result<ProjectCompileOutput<MultiCompiler>>
  where
    F: FnOnce(
      &Project<MultiCompiler>,
    ) -> std::result::Result<
      ProjectCompileOutput<MultiCompiler>,
      foundry_compilers::error::SolcError,
    >,
  {
    if config.language.is_solc_language() {
      solc::ensure_installed(&config.solc_version)?;
    } else if config.language == CompilerLanguage::Vyper {
      vyper::ensure_installed(config.vyper_settings.path.clone())?;
    }
    let project = map_err_with_context(
      build_project(config, self.context),
      "Failed to configure Solidity project",
    )?;
    map_err_with_context(compile_fn(&project), label)
  }

  fn write_virtual_source(&self, config: &CompilerConfig, contents: &str) -> Result<PathBuf> {
    let extension = match config.language {
      crate::internal::config::CompilerLanguage::Solidity => "sol",
      crate::internal::config::CompilerLanguage::Yul => "yul",
      crate::internal::config::CompilerLanguage::Vyper => "vy",
    };

    let source_hash = FoundrySource::content_hash_of(contents);
    let path = self.context.virtual_source_path(&source_hash, extension)?;
    if !path.exists() {
      std::fs::write(&path, contents).map_err(|err| {
        Error::new(format!(
          "Failed to write virtual source {}: {err}",
          path.display()
        ))
      })?;
    }
    Ok(path)
  }

  pub fn prepare_synthetic_context(config: &CompilerConfig) -> Result<Option<ProjectContext>> {
    if !config.cache_enabled {
      return Ok(None);
    }

    let base_dir = default_cache_dir();

    create_synthetic_context(base_dir.as_path()).map(Some)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::internal::config::CompilerLanguage;
  use crate::internal::project::create_synthetic_context;
  use tempfile::tempdir;

  #[test]
  fn write_virtual_source_uses_language_extension() {
    let temp_dir = tempdir().expect("temp dir");
    let context = create_synthetic_context(temp_dir.path()).expect("context");
    let runner = ProjectRunner::new(&context);

    let mut config = CompilerConfig::default();
    config.language = CompilerLanguage::Solidity;
    let sol_path = runner
      .write_virtual_source(&config, "contract A { function f() external {} }")
      .expect("sol path");
    assert!(sol_path
      .extension()
      .unwrap()
      .to_str()
      .unwrap()
      .ends_with("sol"));
    assert_eq!(
      std::fs::read_to_string(&sol_path).expect("read file"),
      "contract A { function f() external {} }"
    );

    config.language = CompilerLanguage::Yul;
    let yul_path = runner
      .write_virtual_source(&config, "object \"Y\" { code { mstore(0, 0) } }")
      .expect("yul path");
    assert!(yul_path
      .extension()
      .unwrap()
      .to_str()
      .unwrap()
      .ends_with("yul"));
  }

  #[test]
  fn prepare_synthetic_context_respects_cache_flag() {
    let mut config = CompilerConfig::default();
    config.cache_enabled = false;
    assert!(ProjectRunner::prepare_synthetic_context(&config)
      .expect("prepare synthetic")
      .is_none());

    config.cache_enabled = true;
    let context = ProjectRunner::prepare_synthetic_context(&config)
      .expect("context")
      .expect("some context");
    assert!(matches!(context.layout, ProjectLayout::Synthetic));
  }
}
