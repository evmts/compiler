TODO: readme

# Compiler Library

Rust + N-API bridge used by the Shadow toolchain to expose Foundry's Solidity compiler workflow to Node.js, Bun, and WASI runtimes.

## Build Targets

- `nx run compiler:build` produces `libs/compiler/build/` with the JS loader, TypeScript entry point, and per-platform native binaries.
- `nx run compiler:copy-types` validates the curated `.d.ts` files and copies them into `build/`. Run this after a successful build when publishing.
- Rust unit tests live behind `nx run compiler:test:rust`; JavaScript shims run with `nx run compiler:test:js`.

> **Note:** Native `.node` binaries are generated locally and in CI. They are gitignored on purpose; consumers are expected to build or download matching artifacts during release.

## Module Layout

- `ast/` – high level AST helpers that parse source, stitch fragments, and expose helper mutations. See `src/ast/README.md`.
- `compiler/` – compilation pipeline. `CompilerCore` resolves project layouts (Foundry, Hardhat, synthetic), drives `ProjectRunner`, and normalises output for JS.
- `internal/` – shared config parsing, solc orchestration, filesystem discovery, and error helpers consumed by both `ast` and `compiler`.

### Compilation Pipeline

1. **Configuration** – N-API `CompilerConfig` inputs (parsed as `JsCompilerConfigOptions`) and any Rust-side `CompilerConfigOptions` are merged into the resolved `CompilerConfig`.
2. **Context detection** – `CompilerCore::new` optionally loads project metadata via `FoundryAdapter`/`HardhatAdapter` or synthesises an ephemeral workspace for inline sources.
3. **Input selection** – `CompilationInput` handles inline strings, source maps, AST units, or file paths. Mixed inputs are rejected at the binding layer for clarity.
4. **Execution** – `CompilerCore::compile_as` runs against an attached project via `ProjectRunner` when available, otherwise falls back to a "pure" `foundry-compilers` invocation with temporary sources.
5. **Result mapping** – outputs are converted into serialisable `JsCompileOutput`/`ContractArtifact` structs that align with the TypeScript bindings in `build/index.d.ts`.

The `Compiler` N-API class threads this flow into four primary entry points (`compileSource`, `compileSources`, `compileFiles`, `compileProject`) plus helpers for installing `solc` versions and instantiating from known project roots.

## Type Generation Workflow

The AST helpers expose richer data than the automatic N-API generator currently understands. To keep the published package ergonomic we hand-author a small set of TypeScript declaration files in `src/types/`. The `copy-types` script intentionally:

1. Type-checks the `.d.ts` files with `tsc` to catch syntax drift.
2. Copies the vetted files into `build/` so they ship with the package.

We keep this script manual so future maintainers do not delete or auto-generate the declarations. Run it whenever the Rust surface changes or before cutting a release:

```bash
nx run compiler:build
nx run compiler:copy-types
```

## JavaScript Integrations

```ts
import { Compiler } from "@compiler/compiler";

const compiler = new Compiler({
  solcVersion: "0.8.30",
  remappings: ["@openzeppelin/=node_modules/@openzeppelin/"],
});
await compiler.installSolcVersion("0.8.30");

const output = compiler.compileSources({
  "MyContract.sol": "contract MyContract { function x() public {} }",
});

console.log(output.contracts["MyContract.sol"]);
```

Pair AST transforms with compilation by using the `Ast` helper (`src/ast/README.md`) to stitch fragments, then feed the resulting source map or AST back through `compileSources`.

## Contract State Helpers

Rust callers can materialise contract metadata in a single step using the new wrappers:

```rust
use compiler::contract::Contract;
use foundry_compilers::artifacts::contract::Contract as FoundryContract;

fn hydrate(contract: &FoundryContract) -> compiler::Result<()> {
  let mut wrapper = Contract::from_foundry_standard_json("MyContract", contract);
  wrapper.with_address(Some("0xdeadbeef".into()));
  let state = wrapper.into_state();
  assert_eq!(state.name, "MyContract");
  Ok(())
}
```

JavaScript bindings expose the same surface through the exported `Contract` class—no `build()` ceremony required:

```ts
import { Contract } from "@compiler/compiler";

const contract = Contract.fromSolcContractOutput("Example", solcArtifact).withAddress("0x1234").withExtra("tag", { env: "test" });

console.log(contract.toJson());

const manual = new Contract({ name: "Manual" }).withAddress("0xdeadbeef").withExtra("note", "local override");

console.log(manual.toJson());
```
