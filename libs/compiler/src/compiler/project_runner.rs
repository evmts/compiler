use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::input::CompilationInput;
use super::output::{into_core_compile_output, CompileOutput};
use crate::internal::config::CompilerLanguage;
use crate::internal::path::canonicalize_path;
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
use log::{error, info};

struct VirtualSourceEntry<'a> {
  original_path: Option<&'a str>,
  contents: &'a str,
}

pub struct ProjectRunner<'a> {
  context: &'a ProjectContext,
}

const LOG_TARGET: &str = "tevm::compiler.project_runner";

impl<'a> ProjectRunner<'a> {
  pub fn new(context: &'a ProjectContext) -> Self {
    Self { context }
  }

  // Compiling a source map or an individual source will create a "virtual" file in the cache
  // directory so we can delegate to compile_files and let the foundry compiler handle caching
  // from the virtual source
  pub fn compile(
    &self,
    config: &CompilerConfig,
    input: &CompilationInput,
  ) -> Result<Option<CompileOutput>> {
    match input {
      CompilationInput::InlineSource { source } => {
        if matches!(self.context.layout, ProjectLayout::Synthetic) && config.cache_enabled {
          info!(
            target: LOG_TARGET,
            "materialising inline source for synthetic project cache"
          );
          let mut paths = self.write_virtual_sources(
            config,
            [VirtualSourceEntry {
              original_path: None,
              contents: source.as_str(),
            }],
            None,
          )?;
          let path = paths
            .pop()
            .ok_or_else(|| Error::new("Failed to prepare virtual source for inline compilation"))?;
          let output = self.compile_with_project(config, "Compilation failed", |project| {
            project.compile_file(path)
          });
          output.map(|out| Some(into_core_compile_output(out)))
        } else {
          info!(
            target: LOG_TARGET,
            "skipping project compilation for inline source (layout={:?}, cache_enabled={})",
            self.context.layout,
            config.cache_enabled
          );
          Ok(None)
        }
      }
      CompilationInput::FilePaths { paths, .. } => {
        if matches!(self.context.layout, ProjectLayout::Synthetic) && !config.cache_enabled {
          info!(
            target: LOG_TARGET,
            "project compilation bypassed for filesystem paths (cache disabled)"
          );
          return Ok(None);
        }
        info!(
          target: LOG_TARGET,
          "delegating filesystem compilation to project (files={})",
          paths.len()
        );
        let normalized = self.context.normalise_paths(paths.as_slice())?;
        let output = self.compile_with_project(config, "Compilation failed", |project| {
          project.compile_files(normalized)
        });
        output.map(|out| Some(into_core_compile_output(out)))
      }
      CompilationInput::SourceMap {
        sources,
        language_override,
      } => {
        if matches!(self.context.layout, ProjectLayout::Synthetic) && config.cache_enabled {
          info!(
            target: LOG_TARGET,
            "materialising source map entries for project compilation (count={}, language_override={:?})",
            sources.len(),
            language_override
          );
          let files = self.write_virtual_sources(
            config,
            sources.iter().map(|(path, contents)| VirtualSourceEntry {
              original_path: Some(path.as_str()),
              contents: contents.as_str(),
            }),
            *language_override,
          )?;
          let output = self.compile_with_project(config, "Compilation failed", move |project| {
            project.compile_files(files.clone())
          });
          output.map(|out| Some(into_core_compile_output(out)))
        } else {
          info!(
            target: LOG_TARGET,
            "skipping project compilation for source map (layout={:?}, cache_enabled={})",
            self.context.layout,
            config.cache_enabled
          );
          Ok(None)
        }
      }
      CompilationInput::AstUnits { .. } => {
        info!(
          target: LOG_TARGET,
          "project runner skipping AST inputs; Foundry doesn't know how to cache"
        );
        Ok(None)
      }
    }
  }

  pub fn compile_project(&self, config: &CompilerConfig) -> Result<CompileOutput> {
    info!(
      target: LOG_TARGET,
      "compiling full project (layout={:?})",
      self.context.layout
    );
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
    info!(
      target: LOG_TARGET,
      "compiling contract {} (layout={:?})",
      contract_name,
      self.context.layout
    );
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
    let started = Instant::now();
    info!(target: LOG_TARGET, "starting project compilation step: {label}");
    if config.language.is_solc_language() {
      info!(
        target: LOG_TARGET,
        "ensuring solc {} for project compilation",
        config.solc_version
      );
      solc::ensure_installed(&config.solc_version)?;
    } else if config.language == CompilerLanguage::Vyper {
      info!(target: LOG_TARGET, "ensuring vyper compiler for project compilation");
      vyper::ensure_installed(config.vyper_settings.path.clone())?;
    }
    let project = match map_err_with_context(
      build_project(config, self.context),
      "Failed to configure Solidity project",
    ) {
      Ok(project) => {
        info!(
          target: LOG_TARGET,
          "project configuration ready (root={})",
          self.context.root.display()
        );
        project
      }
      Err(err) => {
        error!(
          target: LOG_TARGET,
          "project configuration failed during {label}: {}",
          err
        );
        return Err(err);
      }
    };

    match map_err_with_context(compile_fn(&project), label) {
      Ok(output) => {
        info!(
          target: LOG_TARGET,
          "project compilation step succeeded ({label}) in {:?}",
          started.elapsed()
        );
        Ok(output)
      }
      Err(err) => {
        error!(
          target: LOG_TARGET,
          "project compilation step failed ({label}): {}",
          err
        );
        Err(err)
      }
    }
  }

  pub fn prepare_synthetic_context(config: &CompilerConfig) -> Result<Option<ProjectContext>> {
    if !config.cache_enabled {
      info!(
        target: LOG_TARGET,
        "synthetic workspace disabled (cache_enabled=false)"
      );
      return Ok(None);
    }

    let base_dir = default_cache_dir();
    info!(
      target: LOG_TARGET,
      "preparing synthetic workspace under {}",
      base_dir.display()
    );

    create_synthetic_context(base_dir.as_path()).map(|context| {
      info!(
        target: LOG_TARGET,
        "synthetic workspace ready at {}",
        context.root.display()
      );
      Some(context)
    })
  }

  fn write_virtual_sources<'entries, I>(
    &self,
    config: &CompilerConfig,
    entries: I,
    language_override: Option<CompilerLanguage>,
  ) -> Result<Vec<PathBuf>>
  where
    I: IntoIterator<Item = VirtualSourceEntry<'entries>>,
  {
    let mut paths = Vec::new();
    let mut processed = 0usize;

    for entry in entries {
      processed += 1;
      let language = language_override.unwrap_or(config.language);
      let extension = determine_extension(entry.original_path, language);
      let contents = entry.contents;

      let source_hash = FoundrySource::content_hash_of(contents);
      let path = self.context.virtual_source_path(&source_hash, &extension)?;

      fs::create_dir_all(
        path
          .parent()
          .ok_or_else(|| Error::new("Virtual source path missing parent directory"))?,
      )
      .map_err(|err| {
        Error::new(format!(
          "Failed to prepare virtual source directory {}: {err}",
          path.display()
        ))
      })?;

      fs::write(&path, contents).map_err(|err| {
        Error::new(format!(
          "Failed to write virtual source {}: {err}",
          path.display()
        ))
      })?;

      info!(
        target: LOG_TARGET,
        "virtual source prepared (language={:?}, path={})",
        language,
        path.display()
      );
      paths.push(canonicalize_path(&path));
    }

    info!(
      target: LOG_TARGET,
      "materialised {processed} virtual source(s)"
    );
    Ok(paths)
  }
}

fn determine_extension(original_path: Option<&str>, language: CompilerLanguage) -> String {
  if let Some(path) = original_path {
    if let Some(ext) = Path::new(path)
      .extension()
      .and_then(|ext| ext.to_str())
      .filter(|ext| !ext.is_empty())
    {
      return ext.to_string();
    }
  }

  match language {
    CompilerLanguage::Solidity => "sol".to_string(),
    CompilerLanguage::Yul => "yul".to_string(),
    CompilerLanguage::Vyper => "vy".to_string(),
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
      .write_virtual_sources(
        &config,
        [VirtualSourceEntry {
          original_path: None,
          contents: "contract A { function f() external {} }",
        }],
        None,
      )
      .expect("sol path");
    assert!(sol_path
      .last()
      .unwrap()
      .extension()
      .unwrap()
      .to_str()
      .unwrap()
      .ends_with("sol"));
    assert_eq!(
      std::fs::read_to_string(&sol_path.last().unwrap()).expect("read file"),
      "contract A { function f() external {} }"
    );

    config.language = CompilerLanguage::Yul;
    let yul_path = runner
      .write_virtual_sources(
        &config,
        [VirtualSourceEntry {
          original_path: None,
          contents: "object \"Y\" { code { mstore(0, 0) } }",
        }],
        None,
      )
      .expect("yul path");
    assert!(yul_path
      .last()
      .unwrap()
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
