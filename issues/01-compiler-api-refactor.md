# Compiler API Refactor

## Scope
Large-scale restructuring of the `Compiler` façade so the public surface is intuitive, duplication is removed, and the Rust core is reusable outside of the NAPI bindings.

## Problems
- `compile_source`, `compile_sources`, and `compile_files` each re-implement input sorting and validation, letting behavioural differences creep in (`libs/compiler/src/compiler.rs`).
- Project-aware compilation, inline caching, and "pure" compilation live in the same impl, coupling synthetic caching to every call (`libs/compiler/src/compiler.rs:92`, `:415`).
- Returned artifacts are eagerly serialised into strings, forcing consumers to reparse ABI/bytecode (`libs/compiler/src/compile/output.rs`).
- The `from_foundry_root` / `from_hardhat_root` constructors hide which overrides and contexts are applied (`libs/compiler/src/compiler.rs:144`, `:160`).
- JS bindings and business logic are interwoven, preventing internal reuse of compilation helpers without the NAPI layer (`libs/compiler/src/compiler.rs`, `libs/compiler/src/ast.rs:207`).

## Proposed Direction
1. **Introduce a `CompilationInput` enum** capturing inline text, path maps, AST payloads, and filesystem sets. The enum acts as the single internal entry point so every public method simply converts arguments and forwards them.
   ```rust
   // lives in libs/compiler/src/compiler/input.rs (new)
   pub enum CompilationInput {
     InlineSource { source: String },
     SourceMap { sources: BTreeMap<String, String> },
     AstUnits { units: BTreeMap<String, SourceUnit> },
     FilePaths { paths: Vec<PathBuf> },
   }

   impl Compiler {
     #[napi]
     pub fn compile_source(&self, env: Env, target: Either<String, JsObject>, options: Option<JsUnknown>) -> Result<CompileOutput> {
       let input = match target {
         Either::A(source) => CompilationInput::InlineSource { source },
         Either::B(object) => {
           let unit: SourceUnit = env.from_js_value(object.into_unknown())?;
           CompilationInput::AstUnits { units: BTreeMap::from([("__VIRTUAL__.sol".into(), unit)]) }
         }
       };
       self.compile_input(env, input, options)
     }
   }
   ```
   The shared `compile_input` function then performs language inference and routing once, guaranteeing consistent behaviour across `compile_source`, `compile_sources`, and `compile_files`.
2. **Extract a `ProjectRunner` (or similar) type** responsible for translating a `CompilationInput` into Foundry project calls when a `ProjectContext` exists. This runner owns virtual-source caching, `Project::compile_*` calls, and context-specific features, while `Compiler` merely chooses between runner vs pure compilation. This isolates project IO from the façade (`libs/compiler/src/compiler.rs:92`, `:415`).
3. **Return structured artifact data** from the conversion helpers. Replace `String`-encoded ABI/bytecode with typed fields (e.g. `serde_json::Value` for ABI, `Bytes` for bytecode) and move stringification into a NAPI wrapper. This keeps Rust callers from immediately reparsing strings (`libs/compiler/src/compile/output.rs:13`).
4. **Surface explicit adapters** by introducing dedicated constructors such as:
   ```rust
   pub struct CompilerWithContext {
     pub compiler: Compiler,
     pub context: ProjectContext,
     pub resolved: ResolvedCompilerConfig,
   }

   pub fn new_from_foundry(root: &Path, options: Option<&CompilerConfig>) -> Result<CompilerWithContext>;
   ```
   This pattern makes the implicit overrides in `from_foundry_root` / `from_hardhat_root` visible and reusable (`libs/compiler/src/compiler.rs:144`, `:160`).
5. **Split NAPI bindings** into thin wrappers (e.g. `bindings.rs`) that translate JS values, call the core Rust API, and map results back. The core compilation logic becomes ordinary Rust functions without `#[napi]` annotations, enabling reuse from Rust tests or future CLIs (`libs/compiler/src/compiler.rs`, `libs/compiler/src/ast.rs:207`).

## Deliverables
- `CompilationInput` enum plus shared resolver (`compile_input`) that subsumes the duplicated routing logic.
- `ProjectRunner` (or similar) encapsulating project-aware compilation and synthetic caching.
- Updated artifact conversion returning typed ABI/bytecode values with matching TypeScript definitions.
- New adapter constructors exposing config + context to consumers.
- Binding module that wraps the core API, keeping `#[napi]` annotations out of business logic.

## Side Effects & Risks
- Requires touching most `compiler.rs` call sites and Bun tests; ensure behavioural parity with existing integration tests.
- The new artifact structure needs thoughtful TypeScript type generation to avoid breaking users.
- Splitting bindings may require adjusting NAPI attribute placements (e.g. wrappers annotated, core stays plain Rust).

## Validation
- Update existing Bun integration tests (`libs/compiler/test/*.spec.ts`) to cover new shapes.
- Run `npx nx run compiler:test` and `npx nx run compiler:test:rust` to confirm parity.
