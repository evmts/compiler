# TEVM Compiler

Rust-powered, N-API backed tooling that exposes Foundry's multi-language compiler stack (Solidity, Yul, Vyper) to JavaScript runtimes. The bindings ship with high-level helpers for stitching AST fragments, compiling inline sources, and hydrating contract metadata without depending on Hardhat or Foundry CLI wrappers.

## Quick Start

1. **Install toolchains**
   - Node.js 18+ and `pnpm` 9+
   - Bun 1.1+ (required to execute the TypeScript test suite)
   - Rust stable toolchain
   - `solc` releases via `Compiler.installSolcVersion(version)` or Foundry's `svm`
   - Optional: `vyper` executable on your `PATH` for Vyper projects
2. **Install dependencies**
   ```bash
   pnpm install
   ```
3. **Build the bindings**
   ```bash
   pnpm build   # builds the Rust bindings and copies curated .d.ts files, type-checks, generates docs
   ```
4. **Run the full test suite**
   ```bash
   pnpm test         # runs Rust, Bun, and TypeScript checks
   ```

## Usage

- Feed `libs/compiler/build/llms.md` to your favorite LLM and ask how to wire the compiler into your workflow—the bundle contains the public surface, curated types, and executable specs.

## Using the Compiler API

### Compile inline sources

```ts
import { Compiler } from '@tevm/compiler'

await Compiler.installSolcVersion('0.8.30')

const compiler = new Compiler({
  solcVersion: '0.8.30',
  remappings: ['@openzeppelin/=node_modules/@openzeppelin/'],
})

const output = compiler.compileSources({
  'Example.sol': `
    // SPDX-License-Identifier: MIT
    pragma solidity ^0.8.20;

    contract Example {
      function ping() external pure returns (string memory) {
        return 'pong';
      }
    }
  `,
})

if (output.hasCompilerErrors()) {
  console.error(output.diagnostics)
} else {
  const artifact = output.artifact.contracts?.Example
  console.log(artifact?.abi)
}
```

### Target existing projects

- `Compiler.fromFoundryRoot(root, options)` loads `foundry.toml`, honours project remappings, and compiles contracts from `src/`, `test/`, or `script/`.
- `Compiler.fromHardhatRoot(root, options)` normalises Hardhat configuration, wiring build-info, cache directories, and library paths automatically.
- `Compiler.fromRoot(root, options)` binds to any layout when you only have filesystem paths, which will help locate cache and virtual sources.
- All constructors expose `compileProject`, `compileContract(name)`, `compileFiles(paths)`, `compileSources(map)` and `compileSource(source)` as synchronous methods backed by Rust—root layout is just an indicator, but `compileProject` and `compileContract` are not available as we do not have a project graph.

Per-call `compile*` options override constructor defaults, letting you toggle optimiser settings, remappings, or solc versions project-wide.

### Manipulate ASTs before compilation

```ts
import { Ast } from '@tevm/compiler'

await Compiler.installSolcVersion('0.8.30')

const ast = new Ast({
  solcVersion: '0.8.30',
  instrumentedContract: 'Example',
})
  .fromSource(contractSource)
  .injectShadow(fragmentSource)      // inject functions or variables
  .exposeInternalFunctions()         // bump private/internal functions to public
  .exposeInternalVariables()         // bump private/internal variables to public
  .validate()                        // optionally validate the parsed ast by compiling it

const stitched = ast.ast()           // SourceUnit ready for compilation

// The same AST will be available inside the compilation result
const output = compiler.compileSource('contract Example { ... }')
const ast = output.artifact.ast
// this above is exactly the same as:
const ast = new Ast(sameCompilerSettings).fromSource('contract Example { ... }')

```

AST helpers only target Solidity—requests for other languages throw with actionable guidance. IDs remain unique after fragment injection, making the resulting tree safe to feed back into the compiler bindings.

### Leverage contract snapshots

The `Contract` helper wraps compiler artifacts in a declarative API:

```ts
import { Contract } from '@tevm/compiler'

const counter = Contract.fromSolcContractOutput('Counter', artifact).withAddress('0xabc...')

console.log(counter.creationBytecode?.hex)
console.log(counter.toJson())        // normalised contract state
```

## Development Workflow

- `pnpm nx run compiler:build` – compile the Rust bindings and emit `libs/compiler/build`.
- `pnpm nx run compiler:post-build` – copy curated `.d.ts` types, verify them with `tsc`, and regenerate `build/llms.md`.
- `pnpm nx run compiler:test:rust` – execute Rust unit tests with cargo.
- `pnpm nx run compiler:test:js` – run Bun-powered integration specs.
- `pnpm nx run compiler:test:typecheck` – ensure the public `.d.ts` surface stays sound.
- `pnpm nx run compiler:lint` / `:format` – run Biome and `cargo fmt` for JS/Rust sources.

Cacheable Nx targets keep local iteration fast; CI mirrors the same commands.

## Repository Layout

- `libs/compiler/src` – Rust sources, grouped into `ast/`, `compiler/`, `contract/`, and `internal/`.
- `libs/compiler/build` – generated JS entry point (`index.js`), curated `.d.ts`, and per-platform `.node` binaries.
- `libs/compiler/test` – Bun specs covering inline compilation, Foundry/Hardhat projects, AST helpers, and TypeScript type guarantees.
- `dist/` – optional staging area for WASM builds.
- `docs/` – reserved for future human-centric guides.

## Troubleshooting

- **Missing solc binaries:** call `Compiler.installSolcVersion(version)` before compiling, or ensure Foundry's `svm` cache is accessible via `SVM_HOME`.
- **Vyper projects:** install `vyper` 0.3+ and make sure it's on `PATH`. The bindings surface actionable errors when the executable cannot be found.
- **Compilation diagnostics:** check `output.diagnostics` for warnings/errors even when `hasCompilerErrors()` returns `false`; downstream tooling can surface them in IDEs.

## License

MIT © TEVM contributors
