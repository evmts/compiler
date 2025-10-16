# Instrument Module

A Rust implementation of the instrumentation pipeline – parse, inject, and transform Solidity ASTs prior to recompilation.

## Overview

Instrument enables parsing incomplete Solidity code (functions, variables, etc.) and stitching them into existing contracts without requiring semantic validity upfront. This demonstrates that Solidity's parser performs pure syntax analysis, allowing AST manipulation before semantic validation.

## Architecture

```
instrument/
├── lib.rs          # Main Instrument struct and NAPI bindings
├── error.rs        # Error types
├── parser.rs       # Parsing utilities (wrap, parse, analyse)
├── stitcher.rs     # AST stitching logic
├── utils.rs        # JSON utilities (find max ID, renumber, etc.)
└── README.md       # This file
```

## Core Concepts

### 1. Parsing without Compilation

Instrument wraps incomplete Solidity code in minimal boilerplate to make it syntactically valid:

```rust
// Input fragment
let fragment = "function exploit() public view returns (uint) { return secretValue * 2; }";

// Instrument wraps it:
// SPDX-License-Identifier: UNLICENSED
// pragma solidity ^0.8.0;
// contract __InstrumentFragment {
//     function exploit() public view returns (uint) { return secretValue * 2; }
// }
```

### 2. AST Stitching

Instrument extracts nodes from the wrapped AST and stitches them into a target contract:

```
Target AST              Instrument AST          Result AST
┌─────────────┐        ┌──────────────┐        ┌─────────────┐
│ Contract    │        │ Fragment     │        │ Contract    │
│  ├─ getSecret()   +  │  ├─ exploit()    =  │  ├─ getSecret()
│  └─ secret   │        │  └─ ...          │    │  ├─ exploit()
└─────────────┘        └──────────────┘        │  └─ secret   │
                                                └─────────────┘
```

### 3. Re-analysis

After stitching, Instrument re-analyses the combined AST to add semantic information (types, scopes, references).

## API Highlights

```rust
let instrument = Instrument::new(Default::default())
    .from_source(target_source.to_string(), None, None)
    .inject_shadow_source(fragment.to_string(), None)
    .expose_internal_variables(None)
    .expose_internal_functions(None);

let analyzed_ast = instrument.ast();
```

Key entry points:

- `from_source` – hydrate from Solidity source text
- `from_ast` – hydrate from an existing `SourceUnit`
- `inject_shadow_source` / `inject_shadow_ast` – add instrumentation via source or AST fragments
- `expose_internal_variables` / `expose_internal_functions` – promote internal/private members to `public`
- `ast` – retrieve the current AST snapshot for further processing or compilation

## Implementation Details

### Parsing Pipeline

1. **Wrap** – Add boilerplate around instrumentation fragments
2. **Parse** – Use `stopAfter = "parsing"` to obtain the syntax-only AST
3. **Extract** – Pull out fragment members from the synthetic contract
4. **Stitch** – Insert nodes into the target contract, renumbering IDs
5. **Analyse** – Re-compile with `language = "SolidityAST"` via `Compiler::compileAst`

### ID Management

- `find_max_id()` – Recursively finds the highest ID in the target AST
- `renumber_ids()` – Adds an offset to all fragment node IDs to avoid collisions

### Contract Selection

- **Explicit** – Provide `target_contract_name` to select the destination contract
- **Implicit** – When omitted, the last `ContractDefinition` in the source unit is targeted

## Testing

The module is covered by Rust unit tests and Bun-based integration tests. Run the suite via:

```bash
nx test compiler
```

## Known Limitations

1. **Multi-contract files** – Re-analysis of large multi-contract files may fail (solc limitation)
2. **Semantic validity** – Injected code must still pass solc semantic checks when recompiled
3. **Default version** – The default solc version is pinned to `0.8.30`

## Future Improvements

- Support richer instrumentation passes (e.g., marking functions as `virtual`)
- Provide diff-friendly AST output helpers
- Optimise JSON manipulation to minimise cloning
