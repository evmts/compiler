# @tevm/compiler

Rust + N-API bindings that expose Foundry's multi-compiler to JavaScript, Bun, and WASI runtimes. The crate powers TEVM's compiler pipeline and ships with first-class helpers for AST instrumentation and contract state hydration.

## What Lives Here

- `src/ast` – Solidity-only AST orchestration (`Ast` class) for stitching fragments, promoting visibility, and validating stitched trees.
- `src/compiler` – Project-aware compilation core (`Compiler`) that understands Foundry, Hardhat, inline sources, and language overrides (Solidity, Yul, Vyper).
- `src/contract` – Ergonomic wrappers around standard JSON artifacts (`Contract`, `JsContract`) with mutation helpers for downstream tooling.
- `src/internal` – Shared config parsing, solc/vyper orchestration, filesystem discovery, and error translation surfaced through N-API.
- `src/types` – Hand-authored `.d.ts` extensions copied into `build/` after every release.

## Build & Test

```bash
# Build native bindings and emit build/index.{js,d.ts}
pnpm nx run compiler:build

# Copy curated types, generate llms.md, type-check declarations
pnpm nx run compiler:post-build

# Execute the full suite (cargo tests + Bun integration specs + TS type checks)
pnpm nx run compiler:test
```

Useful sub-targets:

- `pnpm nx run compiler:test:rust` – Rust unit tests (`cargo test`).
- `pnpm nx run compiler:test:js` – Bun specs in `test/**/*.spec.ts`.
- `pnpm nx run compiler:test:typecheck` – Validates the published `.d.ts` surface.
- `pnpm nx run compiler:lint` / `:format` – Biome for JS + `cargo fmt` for Rust sources.

## API Highlights

- `Compiler.installSolcVersion(version)` downloads solc releases into the Foundry `svm` cache. `Compiler.isSolcVersionInstalled` performs fast existence checks.
- `new Compiler(options)` compiles inline sources or AST units. `.fromFoundryRoot`, `.fromHardhatRoot`, and `.fromRoot` bootstrap project-aware compilers.
- `compileSource(s)`, `compileFiles`, `compileProject`, `compileContract` return `CompileOutput` snapshots with structured diagnostics, contract wrappers, and standard JSON.
- `Ast` instances parse Solidity sources, inject fragment sources or AST objects (`injectShadow`), expose internal members, and emit unique-ID `SourceUnit`s ready for compilation.
- `Contract` wrappers (available in JS and Rust) provide `.withAddress`, `.withCreationBytecode`, `.withDeployedBytecode`, and `.toJson()` for ergonomic artifact manipulation.

## Release Checklist

1. `pnpm nx run compiler:build --configuration=production`
2. `pnpm nx run compiler:post-build`
3. `pnpm nx run compiler:test`
4. Package platform binaries or publish as required.

The `libs/compiler/build/llms.md` bundle is regenerated automatically during `post-build` so AI assistants stay in sync with the public surface.

## Troubleshooting Notes

- Always call `Compiler.installSolcVersion(version)` (or ensure Foundry's `svm` cache is primed) before running tests locally. Specs assert that required solc versions exist.
- Vyper workflows depend on a `vyper` executable available on `PATH`. Missing binaries throw actionable N-API errors; install via `pipx install vyper`.
- AST helpers reject non-Solidity `solcLanguage` overrides—limit them to Solidity and feed the resulting tree back into `compiler.compileSources`.
