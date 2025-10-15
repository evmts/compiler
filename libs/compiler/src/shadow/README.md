# Shadow Module

A Rust implementation of the Shadow parser - parse and stitch Solidity code fragments into contract ASTs.

## Overview

Shadow enables parsing incomplete Solidity code (functions, variables, etc.) and stitching them into existing contracts without requiring semantic validity upfront. This demonstrates that Solidity's parser performs pure syntax analysis, allowing AST manipulation before semantic validation.

## Architecture

```
shadow/
├── lib.rs          # Main Shadow struct and NAPI bindings
├── error.rs        # Error types
├── parser.rs       # Parsing utilities (wrap, parse, analyze)
├── stitcher.rs     # AST stitching logic
├── utils.rs        # JSON utilities (find max ID, renumber, etc.)
├── tests.rs        # Comprehensive test suite
└── README.md       # This file
```

## Core Concepts

### 1. Parsing without Compilation

Shadow wraps incomplete Solidity code in minimal boilerplate to make it syntactically valid:

```rust
// Input: incomplete function
let shadow_fn = "function exploit() public view returns (uint) { return secretValue * 2; }";

// Shadow wraps it:
// pragma solidity ^0.8.0;
// contract Shadow {
//     function exploit() public view returns (uint) { return secretValue * 2; }
// }
```

### 2. AST Stitching

Shadow extracts nodes from the wrapped AST and stitches them into a target contract:

```
Target AST              Shadow AST              Result AST
┌─────────────┐        ┌─────────────┐        ┌─────────────┐
│ Contract    │        │ Shadow      │        │ Contract    │
│  ├─ getSecret()   +  │  ├─ exploit()    =  │  ├─ getSecret()
│  └─ secret   │        │  └─ ...      │        │  ├─ exploit()
└─────────────┘        └─────────────┘        │  └─ secret   │
                                                └─────────────┘
```

### 3. Re-analysis

After stitching, Shadow re-analyzes the combined AST to add semantic information (types, scopes, references).

## API

### Constructor

```rust
let shadow = Shadow::new(source: String) -> Shadow
```

### Methods

#### `stitch_into_source(target_source, source_name, target_contract_name) -> Result<String>`

Stitch shadow nodes into target source code. Returns fully analyzed AST.

```rust
let shadow = Shadow::new("function exploit() public {}".to_string());
let analyzed_ast = shadow.stitch_into_source(
    target_contract.to_string(),
    None,                    // source_name (default: "Contract.sol")
    None,                    // contract_name (default: last contract)
)?;
```

#### `stitch_into_ast(target_ast_json, target_contract_name, source_name) -> Result<String>`

Stitch shadow nodes into an existing AST. Returns fully analyzed AST.

```rust
let shadow = Shadow::new("uint256 public value;".to_string());
let analyzed_ast = shadow.stitch_into_ast(
    target_ast_json,
    None,  // contract_name
    None,  // source_name
)?;
```

#### `parse_source_ast_static(source, file_name) -> Result<String>`

Static utility to parse Solidity source to AST.

```rust
let ast_json = Shadow::parse_source_ast_static(
    contract_source.to_string(),
    Some("MyContract.sol".to_string()),
)?;
```

## Usage Example

```rust
use compiler::Shadow;

// Create shadow from function fragment
let shadow_fn = "function exploit() public view returns (uint) { return secretValue * 2; }";
let shadow = Shadow::new(shadow_fn.to_string());

// Target contract
let target_source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MyContract {
    uint private secretValue;
    function getSecret() public view returns (uint) { return secretValue; }
}
"#;

// Stitch into source (auto-selects last contract)
let analyzed_ast = shadow.stitch_into_source(
    target_source.to_string(),
    None,
    None,
)?;

// Result: AST with both getSecret() and exploit() functions
```

## Implementation Details

### Parsing Pipeline

1. **Wrap**: Add boilerplate around shadow code
2. **Parse**: Use `stopAfter: "parsing"` to get syntax-only AST
3. **Extract**: Pull out nodes from shadow contract
4. **Stitch**: Insert nodes into target contract, renumbering IDs
5. **Analyze**: Re-import with `language: "SolidityAST"` for full analysis

### ID Management

To prevent ID collisions:

- `find_max_id()`: Recursively finds highest ID in target AST
- `renumber_ids()`: Adds offset to all shadow node IDs

### Contract Selection

- **Explicit**: `target_contract_name: Some("MyContract")` finds by name
- **Heuristic**: `None` uses last `ContractDefinition` in AST

## Testing

Comprehensive test suite with 18 tests covering:

- Shadow creation and wrapping
- AST parsing and extraction
- ID management (find max, renumber)
- Contract finding (by name, last)
- Stitching (into source, into AST)
- Error handling
- Edge cases (multiple nodes, variables, etc.)

Run tests:

```bash
cargo test --lib shadow
```

## Known Limitations

1. **Multi-contract files**: Re-analysis of multi-contract files may fail (solc limitation)
2. **Semantic errors**: Stitched code must be semantically valid for analysis to succeed
3. **Solidity version**: Currently hardcoded to 0.8.30

## Differences from Zig Implementation

1. **No C FFI**: Uses foundry-compilers instead of libsolidity directly
2. **Error handling**: Rust Result types instead of Zig errors
3. **JSON**: serde_json instead of std.json
4. **Memory management**: Rust ownership instead of manual allocation
5. **NAPI bindings**: Direct integration with Node.js via napi-rs

## Future Improvements

- [ ] Support configurable Solidity versions
- [ ] Better error messages with source locations
- [ ] Handle multi-contract file analysis
- [ ] Add `fromAstNodes()` to reconstruct source AST
- [ ] Optimize JSON manipulation (avoid cloning)
- [ ] Add streaming API for large ASTs
