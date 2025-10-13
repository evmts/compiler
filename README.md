# Compiler Monorepo

A monorepo containing Solidity compiler tooling with a focus on pure syntax parsing and AST manipulation.

## Projects

### Shadow - Pure Syntax Parser for Solidity

Shadow is a Zig wrapper around the Solidity Parser that demonstrates **the parser is a pure syntax parser** - it creates ASTs without requiring semantic validity. See [SHADOW_README.md](./SHADOW_README.md) for detailed documentation.

#### Key Features

- ✅ Parse Solidity functions with undefined variables
- ✅ Parse functions with type mismatches
- ✅ Parse functions calling non-existent functions
- ✅ Get full ASTs for code that will never compile
- ✅ Stitch shadow functions into valid contracts
- ✅ Export to WASM for browser use

#### Quick Start

```bash
# Build everything
zig build

# Run demo (shows AST stitching)
zig build run

# Run tests
zig build test

# Build WASM
zig build wasm
```

### Compiler Library (Rust/NAPI)

Located in `packages/compiler` - a comprehensive NAPI-rs wrapper for foundry-compilers with Bun test suite.

## Repository Structure

```
compiler/
├── solidity/              # Solidity compiler source (submodule)
├── shadow.zig            # Shadow parser implementation
├── shadow_test.zig       # Test suite
├── solidity-parser-wrapper.{h,cpp}  # C++ wrapper for parser
├── build.zig             # Build system
├── packages/
│   └── compiler/         # Rust NAPI library
└── apps/                 # Applications
```

## What Shadow Proves

The Solidity compiler has two distinct phases:

1. **Parsing** (syntax-only) - Creates AST from tokens
2. **Analysis** (semantic) - Type checking, variable resolution, etc.

Shadow uses **only the parser**, bypassing all semantic analysis. The parser at [`libsolidity/parsing/Parser.cpp:1112`](https://github.com/argotorg/solidity/blob/a6945de0b/libsolidity/parsing/Parser.cpp#L1112-L1118) just creates `Identifier` nodes without resolution:

```cpp
ASTPointer<Identifier> Parser::parseIdentifier() {
    return nodeFactory.createNode<Identifier>(expectIdentifierToken());
}
```

**No variable resolution. No type checking. No semantic analysis.**

That all happens later in [`CompilerStack::analyze()`](https://github.com/argotorg/solidity/blob/a6945de0b/libsolidity/interface/CompilerStack.cpp#L106-L185) which Shadow bypasses completely!

## Use Cases

- **IDE Features** - Syntax highlighting, structure view without compilation
- **AST Manipulation** - Add functions to contracts without semantic checks
- **Code Analysis** - Analyze structure of invalid/incomplete code
- **Testing** - Parse test fixtures that don't need to compile
- **Browser Tools** - WASM-based Solidity AST explorer

## Building

### Prerequisites

- Zig 0.11+
- C++ compiler (for Solidity parser)
- Node.js (for Rust/NAPI library)

### Build Commands

```bash
# Build native
zig build

# Run Shadow demo
zig build run

# Run tests
zig build test

# Build WASM module
zig build wasm

# Build Rust library
cd packages/compiler
npm install
npm run build
```

## Development

This is an Nx monorepo. Use Nx commands to manage projects:

```bash
# Build specific project
npx nx build <project>

# Run tests
npx nx test <project>

# See dependency graph
npx nx graph
```

## Contributing

This repository demonstrates compiler internals and parser capabilities. Contributions welcome!

## License

- Shadow: GPL-3.0 (same as Solidity)
- Compiler library: See packages/compiler/LICENSE

## Learn More

- [Shadow Documentation](./SHADOW_README.md)
- [Solidity Parser Source](https://github.com/argotorg/solidity)
- [Nx Documentation](https://nx.dev)
