#[macro_use]
extern crate napi_derive;

use foundry_compilers::{
    Project, ProjectBuilder, ProjectPathsConfig,
    solc::{SolcCompiler, SolcLanguage},
    Artifact,
};
use napi::bindgen_prelude::*;
use std::path::PathBuf;

// ============================================================================
// ProjectPathsConfig Wrapper
// ============================================================================

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub root: String,
    pub cache: String,
    pub artifacts: String,
    pub sources: String,
    pub tests: String,
    pub scripts: String,
    pub libraries: Vec<String>,
}

#[napi]
pub fn create_hardhat_paths(root_path: String) -> Result<ProjectPaths> {
    let root = PathBuf::from(&root_path);
    let config: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::hardhat(&root).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to create hardhat paths: {}", e),
        )
    })?;

    Ok(ProjectPaths {
        root: config.root.to_string_lossy().to_string(),
        cache: config.cache.to_string_lossy().to_string(),
        artifacts: config.artifacts.to_string_lossy().to_string(),
        sources: config.sources.to_string_lossy().to_string(),
        tests: config.tests.to_string_lossy().to_string(),
        scripts: config.scripts.to_string_lossy().to_string(),
        libraries: config
            .libraries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    })
}

#[napi]
pub fn create_dapptools_paths(root_path: String) -> Result<ProjectPaths> {
    let root = PathBuf::from(&root_path);
    let config: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::dapptools(&root).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to create dapptools paths: {}", e),
        )
    })?;

    Ok(ProjectPaths {
        root: config.root.to_string_lossy().to_string(),
        cache: config.cache.to_string_lossy().to_string(),
        artifacts: config.artifacts.to_string_lossy().to_string(),
        sources: config.sources.to_string_lossy().to_string(),
        tests: config.tests.to_string_lossy().to_string(),
        scripts: config.scripts.to_string_lossy().to_string(),
        libraries: config
            .libraries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    })
}

#[napi]
pub fn create_current_hardhat_paths() -> Result<ProjectPaths> {
    let config: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::current_hardhat().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to create current hardhat paths: {}", e),
        )
    })?;

    Ok(ProjectPaths {
        root: config.root.to_string_lossy().to_string(),
        cache: config.cache.to_string_lossy().to_string(),
        artifacts: config.artifacts.to_string_lossy().to_string(),
        sources: config.sources.to_string_lossy().to_string(),
        tests: config.tests.to_string_lossy().to_string(),
        scripts: config.scripts.to_string_lossy().to_string(),
        libraries: config
            .libraries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    })
}

#[napi]
pub fn create_current_dapptools_paths() -> Result<ProjectPaths> {
    let config: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::current_dapptools().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to create current dapptools paths: {}", e),
        )
    })?;

    Ok(ProjectPaths {
        root: config.root.to_string_lossy().to_string(),
        cache: config.cache.to_string_lossy().to_string(),
        artifacts: config.artifacts.to_string_lossy().to_string(),
        sources: config.sources.to_string_lossy().to_string(),
        tests: config.tests.to_string_lossy().to_string(),
        scripts: config.scripts.to_string_lossy().to_string(),
        libraries: config
            .libraries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    })
}

// ============================================================================
// Compiler Output Types
// ============================================================================

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompilerError {
    pub message: String,
    pub severity: String,
    pub source_location: Option<SourceLocation>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub start: i32,
    pub end: i32,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ContractArtifact {
    pub contract_name: String,
    pub abi: Option<String>,
    pub bytecode: Option<String>,
    pub deployed_bytecode: Option<String>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub artifacts: Vec<ContractArtifact>,
    pub errors: Vec<CompilerError>,
    pub has_compiler_errors: bool,
}

// ============================================================================
// SolidityProject - Main Project Wrapper
// ============================================================================

#[napi]
pub struct SolidityProject {
    project: Project<SolcCompiler>,
}

#[napi]
impl SolidityProject {
    /// Create a new project from a root path using Hardhat layout
    #[napi(factory)]
    pub fn from_hardhat_root(root_path: String) -> Result<Self> {
        let root = PathBuf::from(&root_path);
        let paths: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::hardhat(&root).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to create hardhat paths: {}", e),
            )
        })?;

        let project = ProjectBuilder::default()
            .paths(paths)
            .build(SolcCompiler::default())
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to build project: {}", e),
                )
            })?;

        Ok(SolidityProject { project })
    }

    /// Create a new project from a root path using Dapptools layout
    #[napi(factory)]
    pub fn from_dapptools_root(root_path: String) -> Result<Self> {
        let root = PathBuf::from(&root_path);
        let paths: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::dapptools(&root).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to create dapptools paths: {}", e),
            )
        })?;

        let project = ProjectBuilder::default()
            .paths(paths)
            .build(SolcCompiler::default())
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to build project: {}", e),
                )
            })?;

        Ok(SolidityProject { project })
    }

    /// Compile all contracts in the project
    #[napi]
    pub fn compile(&self) -> Result<CompileOutput> {
        let output = self.project.compile().map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Compilation failed: {}", e),
            )
        })?;

        let mut artifacts = Vec::new();
        for (name, artifact) in output.artifacts() {
            let abi_json = artifact.abi.as_ref().map(|abi| {
                serde_json::to_string(abi).unwrap_or_default()
            });

            let bytecode = artifact.get_bytecode_bytes().map(|bytes| {
                format!("0x{}", hex::encode(bytes.as_ref()))
            });

            let deployed_bytecode = artifact.get_deployed_bytecode_bytes().map(|bytes| {
                format!("0x{}", hex::encode(bytes.as_ref()))
            });

            artifacts.push(ContractArtifact {
                contract_name: name.clone(),
                abi: abi_json,
                bytecode,
                deployed_bytecode,
            });
        }

        let errors: Vec<CompilerError> = output
            .output()
            .errors
            .iter()
            .map(|err| CompilerError {
                message: err.message.clone(),
                severity: format!("{:?}", err.severity),
                source_location: err.source_location.as_ref().map(|loc| SourceLocation {
                    file: loc.file.clone(),
                    start: loc.start,
                    end: loc.end,
                }),
            })
            .collect();

        Ok(CompileOutput {
            artifacts,
            has_compiler_errors: output.has_compiler_errors(),
            errors,
        })
    }

    /// Compile a single file
    #[napi]
    pub fn compile_file(&self, file_path: String) -> Result<CompileOutput> {
        let path = PathBuf::from(file_path);
        let output = self.project.compile_file(&path).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Compilation failed: {}", e),
            )
        })?;

        let mut artifacts = Vec::new();
        for (name, artifact) in output.artifacts() {
            let abi_json = artifact.abi.as_ref().map(|abi| {
                serde_json::to_string(abi).unwrap_or_default()
            });

            let bytecode = artifact.get_bytecode_bytes().map(|bytes| {
                format!("0x{}", hex::encode(bytes.as_ref()))
            });

            let deployed_bytecode = artifact.get_deployed_bytecode_bytes().map(|bytes| {
                format!("0x{}", hex::encode(bytes.as_ref()))
            });

            artifacts.push(ContractArtifact {
                contract_name: name.clone(),
                abi: abi_json,
                bytecode,
                deployed_bytecode,
            });
        }

        let errors: Vec<CompilerError> = output
            .output()
            .errors
            .iter()
            .map(|err| CompilerError {
                message: err.message.clone(),
                severity: format!("{:?}", err.severity),
                source_location: err.source_location.as_ref().map(|loc| SourceLocation {
                    file: loc.file.clone(),
                    start: loc.start,
                    end: loc.end,
                }),
            })
            .collect();

        Ok(CompileOutput {
            artifacts,
            has_compiler_errors: output.has_compiler_errors(),
            errors,
        })
    }

    /// Compile multiple files
    #[napi]
    pub fn compile_files(&self, file_paths: Vec<String>) -> Result<CompileOutput> {
        let paths: Vec<PathBuf> = file_paths.iter().map(PathBuf::from).collect();
        let output = self.project.compile_files(paths).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Compilation failed: {}", e),
            )
        })?;

        let mut artifacts = Vec::new();
        for (name, artifact) in output.artifacts() {
            let abi_json = artifact.abi.as_ref().map(|abi| {
                serde_json::to_string(abi).unwrap_or_default()
            });

            let bytecode = artifact.get_bytecode_bytes().map(|bytes| {
                format!("0x{}", hex::encode(bytes.as_ref()))
            });

            let deployed_bytecode = artifact.get_deployed_bytecode_bytes().map(|bytes| {
                format!("0x{}", hex::encode(bytes.as_ref()))
            });

            artifacts.push(ContractArtifact {
                contract_name: name.clone(),
                abi: abi_json,
                bytecode,
                deployed_bytecode,
            });
        }

        let errors: Vec<CompilerError> = output
            .output()
            .errors
            .iter()
            .map(|err| CompilerError {
                message: err.message.clone(),
                severity: format!("{:?}", err.severity),
                source_location: err.source_location.as_ref().map(|loc| SourceLocation {
                    file: loc.file.clone(),
                    start: loc.start,
                    end: loc.end,
                }),
            })
            .collect();

        Ok(CompileOutput {
            artifacts,
            has_compiler_errors: output.has_compiler_errors(),
            errors,
        })
    }

    /// Find the path of a contract by its name
    #[napi]
    pub fn find_contract_path(&self, contract_name: String) -> Result<String> {
        let path = self
            .project
            .find_contract_path(&contract_name)
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to find contract: {}", e),
                )
            })?;

        Ok(path.to_string_lossy().to_string())
    }

    /// Get all source files in the project
    #[napi]
    pub fn get_sources(&self) -> Result<Vec<String>> {
        let sources = self.project.sources().map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to get sources: {}", e),
            )
        })?;

        Ok(sources
            .keys()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    }
}

// ============================================================================
// ProjectBuilder Wrapper
// ============================================================================

#[napi]
pub struct SolidityProjectBuilder {
    builder: ProjectBuilder<SolcCompiler>,
}

#[napi]
impl SolidityProjectBuilder {
    /// Create a new project builder
    #[napi(constructor)]
    pub fn new() -> Self {
        SolidityProjectBuilder {
            builder: ProjectBuilder::default(),
        }
    }

    /// Set the project paths using hardhat layout
    #[napi]
    pub fn hardhat_paths(&mut self, root_path: String) -> Result<()> {
        let root = PathBuf::from(root_path);
        let paths: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::hardhat(&root).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to create hardhat paths: {}", e),
            )
        })?;

        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .paths(paths);
        Ok(())
    }

    /// Set the project paths using dapptools layout
    #[napi]
    pub fn dapptools_paths(&mut self, root_path: String) -> Result<()> {
        let root = PathBuf::from(root_path);
        let paths: ProjectPathsConfig<SolcLanguage> = ProjectPathsConfig::dapptools(&root).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to create dapptools paths: {}", e),
            )
        })?;

        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .paths(paths);
        Ok(())
    }

    /// Enable ephemeral mode (disable caching)
    #[napi]
    pub fn ephemeral(&mut self) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .ephemeral();
        self
    }

    /// Set cached mode
    #[napi]
    pub fn set_cached(&mut self, cached: bool) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .set_cached(cached);
        self
    }

    /// Enable offline mode
    #[napi]
    pub fn offline(&mut self) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .offline();
        self
    }

    /// Set offline mode
    #[napi]
    pub fn set_offline(&mut self, offline: bool) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .set_offline(offline);
        self
    }

    /// Disable writing artifacts to disk
    #[napi]
    pub fn no_artifacts(&mut self) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .no_artifacts();
        self
    }

    /// Set whether to write artifacts
    #[napi]
    pub fn set_no_artifacts(&mut self, no_artifacts: bool) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .set_no_artifacts(no_artifacts);
        self
    }

    /// Set the number of parallel solc jobs
    #[napi]
    pub fn solc_jobs(&mut self, jobs: u32) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .solc_jobs(jobs as usize);
        self
    }

    /// Limit to single solc job
    #[napi]
    pub fn single_solc_jobs(&mut self) -> &Self {
        self.builder = std::mem::replace(&mut self.builder, ProjectBuilder::default())
            .single_solc_jobs();
        self
    }

    /// Build the project
    #[napi]
    pub fn build(&mut self) -> Result<SolidityProject> {
        let builder = std::mem::replace(&mut self.builder, ProjectBuilder::default());
        let project = builder.build(SolcCompiler::default()).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to build project: {}", e),
            )
        })?;

        Ok(SolidityProject { project })
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
    a + b
}

#[napi]
pub fn find_artifacts_dir(root_path: String) -> String {
    let root = PathBuf::from(root_path);
    let artifacts_dir = ProjectPathsConfig::find_artifacts_dir(&root);
    artifacts_dir.to_string_lossy().to_string()
}

#[napi]
pub fn find_source_dir(root_path: String) -> String {
    let root = PathBuf::from(root_path);
    let source_dir = ProjectPathsConfig::find_source_dir(&root);
    source_dir.to_string_lossy().to_string()
}

#[napi]
pub fn find_libs(root_path: String) -> Vec<String> {
    let root = PathBuf::from(root_path);
    let libs = ProjectPathsConfig::find_libs(&root);
    libs.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}
