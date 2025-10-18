use std::path::PathBuf;

use foundry_compilers::artifacts::sources::Source as FoundrySource;
use foundry_compilers::artifacts::SolcLanguage as FoundrySolcLanguage;
use foundry_compilers::solc::SolcCompiler;
use foundry_compilers::{Project, ProjectCompileOutput};
use std::sync::OnceLock;

use super::input::CompilationInput;
use super::output::{into_core_compile_output, CoreCompileOutput};
use crate::internal::{
  config::ResolvedCompilerConfig,
  errors::{map_err_with_context, Error, Result},
  project::{build_project, ProjectContext, ProjectLayout},
  solc,
};

pub struct ProjectRunner<'a> {
  context: &'a ProjectContext,
}

impl<'a> ProjectRunner<'a> {
  pub fn new(context: &'a ProjectContext) -> Self {
    Self { context }
  }

  pub fn compile(
    &self,
    config: &ResolvedCompilerConfig,
    input: &CompilationInput,
  ) -> Result<Option<CoreCompileOutput>> {
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
        let normalized = self.context.normalise_paths(config, paths.as_slice())?;
        let output = self.compile_with_project(config, "Compilation failed", |project| {
          project.compile_files(normalized)
        });
        output.map(|out| Some(into_core_compile_output(out)))
      }
      CompilationInput::SourceMap { .. } | CompilationInput::AstUnits { .. } => Ok(None),
    }
  }

  pub fn compile_project(&self, config: &ResolvedCompilerConfig) -> Result<CoreCompileOutput> {
    let output = self.compile_with_project(config, "Project compilation failed", |project| {
      project.compile()
    });
    output.map(into_core_compile_output)
  }

  pub fn compile_contract(
    &self,
    config: &ResolvedCompilerConfig,
    contract_name: &str,
  ) -> Result<CoreCompileOutput> {
    let name = contract_name.to_owned();
    let output = self.compile_with_project(config, "Contract compilation failed", move |project| {
      let path = project.find_contract_path(&name)?;
      project.compile_file(path)
    });
    output.map(into_core_compile_output)
  }

  fn compile_with_project<F>(
    &self,
    config: &ResolvedCompilerConfig,
    label: &str,
    compile_fn: F,
  ) -> Result<ProjectCompileOutput<SolcCompiler>>
  where
    F: FnOnce(
      &Project<SolcCompiler>,
    ) -> std::result::Result<
      ProjectCompileOutput<SolcCompiler>,
      foundry_compilers::error::SolcError,
    >,
  {
    solc::ensure_installed(&config.solc_version)?;
    let project = map_err_with_context(
      build_project(config, self.context),
      "Failed to configure Solidity project",
    )?;
    map_err_with_context(compile_fn(&project), label)
  }

  fn write_virtual_source(
    &self,
    config: &ResolvedCompilerConfig,
    contents: &str,
  ) -> Result<PathBuf> {
    let extension = match config.solc_language {
      FoundrySolcLanguage::Solidity => "sol",
      FoundrySolcLanguage::Yul => "yul",
      _ => "sol",
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

  pub fn prepare_synthetic_context(
    config: &mut ResolvedCompilerConfig,
  ) -> Result<Option<ProjectContext>> {
    if !config.cache_enabled {
      return Ok(None);
    }

    let base_dir = match config.base_dir.clone() {
      Some(dir) => dir,
      None => {
        let default_dir = default_cache_dir();
        config.base_dir = Some(default_dir.clone());
        default_dir
      }
    };

    crate::internal::project::create_synthetic_context(base_dir.as_path()).map(Some)
  }
}

fn default_cache_dir() -> PathBuf {
  static CACHE_PATH: OnceLock<PathBuf> = OnceLock::new();
  CACHE_PATH
    .get_or_init(|| {
      let root = std::env::temp_dir().join(".tevm/cache");
      let _ = std::fs::create_dir_all(&root);
      root
    })
    .clone()
}
