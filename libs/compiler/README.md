# @tevm/compiler

Rust + N-API bindings that expose Foundry's multi-language compiler (Solidity, Yul, Vyper) to JavaScript and Bun runtimes. The package ships with helpers for AST instrumentation, contract state objects with convenient types, and project-aware builds (Foundry, Hardhat, or from a custom root). This allows any project to benefit from Foundry's compiler stack and caching capabilities in a custom structure. This includes caching inline sources.

## Quick Start

1. **Install toolchains**
   - Node.js 18+ with `pnpm` 9+
   - Bun 1.1+ (required for the test suite)
   - Rust stable toolchain
   - Relevant compiler binaries:
     - Install `solc` releases via `Compiler.installSolcVersion(version)` or Foundry's `svm`
     - Optional: `vyper` executable on your `PATH` for Vyper projects
2. **Install dependencies**
   ```bash
   pnpm install
   ```
3. **Build native bindings**
   ```bash
   pnpm nx run compiler:build
   pnpm nx run compiler:post-build   # copies curated .d.ts files, type-checks, regenerates build/llms.txt
   ```
4. **Run the full test matrix**
   ```bash
   pnpm nx run compiler:test         # cargo tests + Bun specs + TS type assertions
   ```

## Usage

- Feed `libs/compiler/build/llms.txt` to your favourite LLM and ask how to adapt the compiler for your workflow—the bundle includes the public API surface, curated `.d.ts`, and executable specs.
- The sections below show direct JavaScript usage patterns; all examples run in Node.js or Bun.
- You will also find realistic use cases in [test/integrations.spec.ts](test/integrations.spec.ts).

### Compile inline sources

```ts
import { Compiler, CompilerLanguage } from '@tevm/compiler'

await Compiler.installSolcVersion('0.8.30')

const compiler = new Compiler({
  language: 'solidity', // or 'yul', 'vyper'
  solcVersion: '0.8.30',
  solcSettings: {
    // any solc settings, see index.d.ts:CompilerSettings
  }

  // or
  language: CompilerLanguage.Vyper,
  vyperSettings: {
    // any vyper settings, see index.d.ts:VyperCompilerSettings
  }
})

// This will be cached by default in ~/.tevm/virtual-sources
const output = compiler.compileSources({
  'Example.sol': `
    // SPDX-License-Identifier: MIT
    pragma solidity ^0.8.20;

    contract Example {
      ...
    }
  `,
}, {
    // override any constructor settings; this is true for every compile method
})

if (output.hasCompilerErrors()) {
  console.error(output.diagnostics)
} else {
  // The artifacts paths are fully typed
  const artifact = output.artifacts["Example.sol"].contracts.Example
  console.log(artifact?.toJson())
}

// Compile a single source, which will be cached as well as a virtual source
const output = compiler.compileSource('contract Example { uint256 private value; }')
const artifact = output.artifact.contract.Example
// or some files
const output = compiler.compileFiles(['Example.sol', 'Another.sol'])
// ...
```

### Target existing projects

```ts
import { Compiler } from "@tevm/compiler";
import { join } from "node:path";

// Reuse foundry.toml configuration, remappings, and cache directories.
const foundryRoot = join(process.cwd(), "projects", "foundry-sample");
const foundryCompiler = Compiler.fromFoundryRoot(foundryRoot, {
  solcVersion: "0.8.30",
});

// Compile everything the project declares in its remappings/sources
const projectSnapshot = foundryCompiler.compileProject();
// Narrow to a single contract that will be resolved with the project graph
const counterSnapshot = foundryCompiler.compileContract("Counter");

// Hardhat projects automatically normalise cache + build-info placement
const hardhatRoot = join(process.cwd(), "projects", "hardhat-sample");
const hardhatCompiler = Compiler.fromHardhatRoot(hardhatRoot);
const compiledHardhat = hardhatCompiler.compileSources({
  "Inline.sol": "contract Inline { function value() public {} }",
});

// Work inside an arbitrary directory while still persisting .tevm artifacts.
const syntheticRoot = join(process.cwd(), "tmp", "inline-only");
const syntheticCompiler = Compiler.fromRoot(syntheticRoot);
// or `new Compiler()` which will use the current workspace as root
const inlineSnapshot = syntheticCompiler.compileSource("contract Foo { }");
```

### Manipulate ASTs for shadowing contracts

```ts
import { Ast, Compiler, ResolveConflictStrategy } from "@tevm/compiler";

await Compiler.installSolcVersion("0.8.30");

const ast = new Ast({
  solcVersion: "0.8.30",
  instrumentedContract: "Example", // this is not necessary if there is only one contract
})
  .fromSource("contract Example { uint256 private value; }")
  .injectShadow("function getValue() public returns (uint256) { return value; }") // any inline Solidity (contract body)
  .exposeInternalFunctions() // promote private/internal functions
  .exposeInternalVariables() // promote private/internal variables
  .validate(); // optional: recompiles to ensure the AST is sound

const stitched = ast.sourceUnit(); // SourceUnit ready for compilation

// Compile the instrumented AST (this will reuse the cached output from validate() if not invalidated)
const compiled = ast.compile();
// which is exactly the same as:
const compiler = new Compiler({ solcVersion: "0.8.30" });
const output = compiler.compileSources({ "Example.sol": stitched });
// The compilation output returns ast classes as well
const ast = output.artifacts["Example.sol"].ast;
```

When a fragment redefines existing members you can switch the conflict strategy to replace the matching node while still appending the rest:

```ts
ast.injectShadow(
  "function getValue() public view returns (uint256) { return value + 1; }",
  // 'safe' is the default strategy (will fail to compile if conflicting members are found)
  // 'replace' will overwrite the existing members when conflicting
  { resolveConflictStrategy: 'replace' },
)
```

For quick instrumentation (e.g. invariants, guards), `injectShadowAtEdges` injects your snippets directly into the original body without changing the function signature. Each `return` path receives the "after" statements and the fallthrough path is automatically covered so the original control-flow remains intact while your instrumentation runs.

```ts
// Inject invariants before and after an existing function body.
new Ast({ solcVersion: "0.8.30", instrumentedContract: "Token" })
  .fromSource(readFileSync("Token.sol", "utf8"))
  .injectShadowAtEdges("mint(address, uint256)", { // signature can be important if there are overloads
    before: "uint256 __totalSupplyBefore = totalSupply();",
    after: "require(totalSupply() == __totalSupplyBefore + amount);",
  })
  .validate();
```

```ts
// Emit a shadow event inside a function
new Ast({ solcVersion: "0.8.30", instrumentedContract: "Token" })
  .fromSource(readFileSync("Token.sol", "utf8"))
  .injectShadow(`
    event BalanceChangeTrace(address account, uint256 balanceAfter);
  `)
  .injectShadowAtEdges("transfer", {
    after: [
        "emit BalanceChangeTrace(msg.sender, balanceOf(msg.sender));",
        "emit BalanceChangeTrace(to, balanceOf(to));",
    ],
  })
  .validate();
```

AST helpers only support Solidity targets; requests for other languages throw with actionable guidance. Node IDs remain unique after fragment injection, making the resulting tree safe to feed back into the compiler.

### Contract snapshots

```ts
import { Contract } from "@tevm/compiler";

const counter = Contract
  .fromSolcContractOutput("Counter", artifact)
  .withAddress("0xabc...")
  .withDeployedBytecode("0x6000...");

// address and deployedBytecode are typed
console.log(counter.address);
console.log(counter.deployedBytecode.hex);
console.log(counter.toJson()); // normalised contract state
```

`CompileOutput` instances expose `.artifacts`, `.artifact`, `.errors`, `.diagnostics`, `.hasCompilerErrors()`, and `.toJson()` so downstream tools can safely persist or transport build metadata.

## Build & Test Commands

```bash
# Build native bindings and emit build/index.{js,d.ts}
pnpm nx run compiler:build

# Copy curated types, generate llms.txt, type-check declarations
pnpm nx run compiler:post-build

# Execute the full suite (cargo tests + Bun integration specs + TS type checks)
pnpm nx run compiler:test
```

Useful sub-targets:

- `pnpm nx run compiler:test:rust` – Rust unit tests (`cargo test`).
- `pnpm nx run compiler:test:js` – Bun specs in `test/**/*.spec.ts`.
- `pnpm nx run compiler:test:typecheck` – Validates the published `.d.ts` surface.
- `pnpm nx run compiler:lint` / `:format` – Biome for JS + `cargo fmt` for Rust sources.

## What Lives Here

- `src/ast` – Solidity-only AST orchestration (`Ast` class) for stitching fragments, promoting visibility, and validating stitched trees.
- `src/compiler` – Project-aware compilation core (`Compiler`) that understands Foundry, Hardhat, inline sources, and language overrides.
- `src/contract` – Ergonomic wrappers around standard JSON artifacts (`Contract`, `JsContract`) with mutation helpers for downstream tooling.
- `src/internal` – Shared config parsing, compiler orchestration, filesystem discovery, and error translation surfaced through N-API.
- `src/types` – Hand-authored `.d.ts` extensions copied into `build/` after every release.
- `test/` – Bun-powered specs and TypeScript assertion suites describing expected behaviour.

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

The `libs/compiler/build/llms.txt` bundle is regenerated automatically during `post-build` so AI assistants stay in sync with the public surface.

## Troubleshooting Notes

- Always call `Compiler.installSolcVersion(version)` (or ensure Foundry's `svm` cache is primed) before running tests locally. Specs assert that required solc versions exist.
- Vyper workflows depend on a `vyper` executable available on `PATH`. Missing binaries throw actionable N-API errors; install via `pipx install vyper`.
- AST helpers reject non-Solidity `solcLanguage` overrides—limit them to Solidity and feed the resulting tree back into `compiler.compileSources`.
