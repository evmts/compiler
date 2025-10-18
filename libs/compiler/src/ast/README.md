# AST Helpers

The `Ast` module wraps Foundry's Solidity AST structures with mutation helpers that Shadow uses to stitch instrumented fragments into a target contract before recompilation.

## Workflow Overview

1. **Initialisation** – `Ast::new` resolves JS `AstConfigOptions` inputs (parsed as `JsAstConfigOptions`) into a resolved `AstConfig`, sanitises compiler settings, ensures the requested `solc` version is installed, and prepares defaults.
2. **Target loading** – `from_source` accepts either Solidity source text or a pre-parsed `SourceUnit`. When a target contract name is provided, the orchestrator verifies it exists inside the unit up front.
3. **Fragment injection** – `inject_shadow` parses Solidity snippets or accepts a pre-built AST fragment, extracts the fragment contract, and stitches it into the target contract at the correct node boundaries.
4. **Post-processing** – `expose_internal_variables` and `expose_internal_functions` promote private/internal members to public visibility to support later JS-level instrumentation.
5. **Export** – `ast()` returns a sanitised `SourceUnit` ready to be serialised back to JS or passed to the compiler bindings.

## Key Components

- `orchestrator.rs` – wraps `foundry-compilers` parsing routines and centralises AST sanitation.
- `parser.rs` – low-level helpers that call into Solc to parse virtual sources and fragments.
- `stitcher.rs` – finds insertion points, merges fragment nodes, and updates visibility as needed.
- `utils.rs` – conversions between `serde_json::Value` and JS values plus AST sanitisation.
- `error.rs` – shared error types surfaced through N-API.

## Example

```ts
import { Ast } from '@tevm/compiler';

const ast = new Ast({
  solcVersion: '0.8.26',
  instrumentedContract: 'Vault',
});

const stitched = ast
  .fromSource(contractSource)
  .injectShadow(fragmentSource)
  .exposeInternalFunctions()
  .ast();
```

The resulting `stitched` value can be passed directly to `Compiler#compileSources` as an AST unit, or serialised back to Solidity source depending on the downstream workflow.
