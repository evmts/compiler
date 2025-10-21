# LLM Context Guide (For Language Models Only)

This repository hosts `@tevm/compiler`, a Rust + N-API bridge that surfaces Foundry's Solidity/Yul/Vyper compiler stack to JavaScript and Bun. The public API lives in `libs/compiler/build/index.js` (with types in `index.d.ts`) and exposes:

- `Compiler` – synchronous bindings for compiling inline strings, AST objects, on-disk files, or full Foundry/Hardhat projects.
- `Ast` – helpers for parsing Solidity sources, injecting fragment code (a.k.a. "shadow" snippets), and returning stitched `SourceUnit` trees ready for recompilation.
- `Contract` & `CompileOutput` – ergonomic wrappers around solc artifacts, diagnostics, and contract metadata.

Tests exist to document behaviour and guard against regressions. Each suite uses Bun's test runner.

## Test Suites

- `ast.spec.ts` – Exercises the `Ast` helper: parsing source text, injecting fragment functions or variables, promoting visibility, and validating ID stability. It also checks error handling for unsupported languages and uninitialised AST access.
- `compiler.spec.ts` – Covers the inline compiler surface. Ensures solc version helpers work, verifies diagnostics for success/warning/error scenarios, validates optimistic batching (`compileSources`, `compileFiles`, `compileProject`), and asserts helper utilities like project path discovery behave correctly for synthetic projects.
- `compiler.foundry.spec.ts` – Clones the Foundry fixture project and verifies project-bound compilation flows: `compileProject`, `compileContract`, per-call overrides, and canonical path resolution.
- `compiler.hardhat.spec.ts` – Mirrors the Foundry suite for Hardhat fixtures, confirming normalised configuration, cache directories, and build-info integration.
- `contract.spec.ts` – Validates the TypeScript-level contract wrapper: mutation helpers (`withAddress`, `withCreationBytecode`, etc.) and JSON serialisation.
- `typecheck.spec.ts` – Contains TypeScript type assertions that encode expectations for the public `.d.ts` surface, ensuring the emitted declarations stay sound.

## Core Concepts

- **Inline Compilation:** `new Compiler({ solcVersion: '0.8.30' }).compileSources({ 'File.sol': source })` returns a `CompileOutput` snapshot with deterministic contract/diagnostic accessors.
- **Project-Aware Compilation:** `Compiler.fromFoundryRoot(root)` and `Compiler.fromHardhatRoot(root)` discover configuration, remappings, and cache paths automatically. The same `compile*` methods return typed outputs; per-call options override constructor defaults.
- **AST Instrumentation:** `new Ast({ instrumentedContract: 'Target', solcVersion: '0.8.30' }).fromSource(source).injectShadow(fragment)` returns a stitched AST for further manipulation or compilation.
- **Contract State Helpers:** `Contract.fromSolcContractOutput(name, artifact)` wraps standard JSON output, allowing selective mutation (`withAddress`, `withCreationBytecode`, etc.) before serialising with `.toJson()`.

Refer to the individual test files for executable examples of each workflow. This guide intentionally omits build commands, Nx targets, and release procedures to keep the focus on the runtime API and its intended usage scenarios.***
