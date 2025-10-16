# Ast Module

Rust utilities for parsing, editing, and re-emitting Solidity Abstract Syntax Trees prior to recompilation.

## Overview

The `Ast` helper loads Solidity source or existing AST JSON, lets you splice in additional nodes, tweak visibility, and then hand the result back to `solc` for a "SolidityAST" round trip. It keeps the original node IDs consistent and normalises nullable fields so the payload matches solc's own shape.

## Layout

```
ast/
├── ast.rs         # `Ast` NAPI surface and core workflow
├── error.rs       # Error types converted into napi::Error
├── parser.rs      # Helpers for wrapping/parsing source fragments
├── stitcher.rs    # Insert fragment nodes into target contracts
├── utils.rs       # JSON helpers (max id, renumber, sanitise)
└── README.md
```

## Typical Flow

```rust
let mut ast = Ast::new(Default::default())
    .from_source(target_source.to_owned(), None)
    .inject_shadow_source(fragment.to_owned(), None)
    .expose_internal_variables(None)
    .expose_internal_functions(None);

let instrumented = ast.get();
```

Key entry points:

- `from_source` / `from_ast` – hydrate the helper
- `inject_shadow_source` / `inject_shadow_ast` – stitch fragment nodes
- `expose_internal_*` – promote private members to `public`
- `get` – retrieve the JSON representation for compilation

## Implementation Notes

1. **Parsing** – uses `stopAfter = "parsing"` to obtain syntax-only `SourceUnit`s.
2. **Stitching** – renumbers fragment IDs before inserting so they remain unique.
3. **Sanitising** – removes `null` fields and restores empty structures solc expects.
4. **Recompilation** – callers feed `get()` into `Compiler::compileAst` to produce bytecode.

## Testing

Both Rust unit tests and Bun tests cover stitching, visibility updates, and the solc round trip:

```bash
pnpm nx run compiler:test:rust
bun test libs/compiler/test/ast.spec.ts
```
