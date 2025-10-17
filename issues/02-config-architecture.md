# Configuration Architecture Cleanup

## Scope
Medium-to-large refactor to streamline compiler configuration resolution, path handling, and environment inference so it is predictable and easier to extend.

## Problems
- `ResolvedCompilerConfig` and `ConfigOverrides` merge logic is verbose and error-prone, spreading option handling across multiple passes (`libs/compiler/src/internal/config.rs:81`).
- Path canonicalisation utilities are repeated in config parsing, project setup, and command helpers (`libs/compiler/src/internal/config.rs:214`, `libs/compiler/src/internal/project.rs:110`).
- Foundry/Hardhat inference lives inside `compiler.rs`, mixing IO-heavy detection with compilation logic (`libs/compiler/src/compiler.rs:580`, `:687`).
- Inline caching helpers (`write_virtual_source`) reach directly into `ProjectContext`, but the context itself does not own any helper APIs (`libs/compiler/src/compiler.rs:503`).
- Settings overlay logic is duplicated between AST and compiler code paths, risking divergence (`libs/compiler/src/internal/config.rs:240`, `libs/compiler/src/internal/settings.rs:23`, `libs/compiler/src/ast.rs:45`).

## Proposed Direction
1. **Adopt a `CompilerConfigBuilder`** that assembles the final config in one pass, eliminating the triple-merge dance. Example flow:
   ```rust
   // libs/compiler/src/internal/config/builder.rs (new)
   pub struct CompilerConfigBuilder {
     base: Settings,
     overrides: CompilerConfig,
   }

   impl CompilerConfigBuilder {
     pub fn build(self) -> Result<ResolvedCompilerConfig> {
       let mut resolved = ResolvedCompilerConfig::from_defaults();
       resolved.solc_version = parse_version(self.overrides.solc_version.as_deref(), &resolved.solc_version)?;
       resolved.solc_language = self.overrides.solc_language.map(Into::into).unwrap_or(resolved.solc_language);
       resolved.solc_settings = merge_settings(&resolved.solc_settings, self.overrides.solc_settings.as_ref())?;
       // ... fill out the rest in one go
       Ok(resolved)
     }
   }
   ```
   This collapses the `ResolvedCompilerConfig::default` → `merge_options` → `merged` chain (`libs/compiler/src/internal/config.rs:81`).
2. **Centralise filesystem helpers** by moving `canonicalize_path`, `to_path_set`, `to_path_vec`, and similar utilities into `internal/path.rs` (new). Both config parsing and project creation (`libs/compiler/src/internal/project.rs:110`) use the same implementations, avoiding drift.
3. **Move Foundry/Hardhat detection into adapters** like `FoundryAdapter::load(root)` and `HardhatAdapter::load(root)`. Each adapter returns `(ResolvedOverrides, ProjectContext)` and lives in `internal/project/adapters/*.rs`. `compiler.rs` stops embedding IO-heavy detection logic inline (`libs/compiler/src/compiler.rs:580`, `:687`).
4. **Extend `ProjectContext`** with helper methods:
   ```rust
   impl ProjectContext {
     pub fn normalise_paths(&self, config: &ResolvedCompilerConfig, inputs: &[String]) -> Result<Vec<PathBuf>>;
     pub fn virtual_source_path(&self, hash: &str, extension: &str) -> Result<PathBuf>;
   }
   ```
   This keeps virtual-source caching and path normalisation close to the context itself (`libs/compiler/src/internal/project.rs:26`, `libs/compiler/src/compiler.rs:503`).
5. **Create a settings merger module** (e.g. `internal/settings/merge.rs`) that exposes `merge_settings(base, overrides)` and `sanitize_settings(settings)`. Both the compiler config and AST code use these helpers, unifying behaviour (`libs/compiler/src/internal/config.rs:240`, `libs/compiler/src/internal/settings.rs:23`, `libs/compiler/src/ast.rs:45`).

## Deliverables
- Builder-based config resolution replacing current override chaining, including unit tests for edge cases (missing versions, invalid severity strings).
- Shared filesystem utility module consumed by both config and project code.
- Adapter modules (`foundry.rs`, `hardhat.rs`) returning typed overrides and contexts.
- Extended `ProjectContext` API covering virtual source and path normalisation duties.
- Reusable settings merger utilities with tests validating equivalence to current behaviour.

## Side Effects & Risks
- Changing config resolution impacts every compile path; thorough regression testing is required.
- Moving adapters will touch `from_foundry_root` / `from_hardhat_root` call sites and tests that assert behaviour.
- Ensure the new utilities remain `no_std`-friendly if ever reused elsewhere (keep dependencies minimal).

## Validation
- Add unit tests for the new builder and filesystem utilities.
- Re-run Bun + Rust integration suites (`npx nx run compiler:test`, `npx nx run compiler:test:rust`).
- Spot-check Foundry and Hardhat fixture tests (`libs/compiler/test/compiler.foundry.spec.ts`, `compiler.hardhat.spec.ts`) for regressions.
