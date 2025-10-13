# Shadow - Pure Syntax Parser for Solidity

Shadow is a Zig wrapper around the Solidity Parser that demonstrates the parser is a **pure syntax parser** - it creates ASTs without requiring semantic validity.

## What This Proves

The Solidity compiler has two distinct phases:

1. **Parsing** (syntax-only) - Creates AST from tokens
2. **Analysis** (semantic) - Type checking, variable resolution, etc.

Shadow uses **only the parser**, bypassing all semantic analysis. This means you can:

- Parse functions with undefined variables ✓
- Parse functions with type mismatches ✓
- Parse functions calling non-existent functions ✓
- Get full ASTs for code that will never compile ✓

## Structure

```
Shadow (Zig struct)
  ├── Takes inline function string
  ├── Wraps with minimal boilerplate
  ├── Calls Parser directly via C++ wrapper
  └── Returns JSON AST
```

## Files

- `shadow.zig` - Main Zig code with Shadow struct
- `solidity-parser-wrapper.h/cpp` - C wrapper for C++ Parser
- `build.zig` - Build configuration for native and WASM

## Building

### Native Build

```bash
zig build
```

### Run Demo

```bash
zig build run
```

This will run three demos:
1. Function with undefined variable
2. Function with type mismatch
3. Function calling non-existent function

All three will **successfully parse** despite being semantically invalid!

### Run Tests

```bash
zig build test
```

Runs comprehensive test suite:
- ✓ Parse function with undefined variable
- ✓ Parse function with type mismatch
- ✓ Parse function calling non-existent function
- ✓ Parse function with multiple undefined variables
- ✓ Parse function with invalid struct access
- ✓ Parse valid function (for comparison)
- ✓ Verify syntax errors properly fail
- ✓ Memory leak checks

### WASM Build

```bash
zig build wasm
```

Output: `zig-out/lib/libshadow-wasm.a`

## Usage Example

```zig
const shadow = try Shadow.init(allocator,
    \\function test() public {
    \\    return undefinedVar + 5;
    \\}
);
defer shadow.deinit();

// This works even though undefinedVar doesn't exist!
const ast_json = try shadow.parseToAST();
defer allocator.free(ast_json);

std.debug.print("AST: {s}\n", .{ast_json});
```

## How It Works

1. **Shadow.init()** - Takes function string, creates parser context
2. **wrapFunction()** - Adds minimal boilerplate:
   ```solidity
   pragma solidity ^0.8.0;
   contract Shadow {
       <your function here>
   }
   ```
3. **parseToAST()** - Calls `Parser::parse()` directly
4. Returns JSON AST via `ASTJsonExporter`

## Key Insight

The parser at `libsolidity/parsing/Parser.cpp:1112` just creates `Identifier` nodes:

```cpp
ASTPointer<Identifier> Parser::parseIdentifier() {
    return nodeFactory.createNode<Identifier>(expectIdentifierToken());
}
```

**No variable resolution. No type checking. No semantic analysis.**

That all happens later in `CompilerStack::analyze()` which we bypass completely!

## Why This Matters

The `solc` npm package doesn't expose parse-only mode, but the **underlying parser supports it**. Shadow proves this by using the parser directly.

## Next Steps

- Export Shadow as WASM module
- Create JavaScript bindings
- Build browser-based AST explorer for invalid Solidity code
- Use for IDE features (syntax highlighting, structure view) without compilation

## License

Same as Solidity (GPL-3.0)
