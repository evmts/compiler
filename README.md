# Shadow - Solidity AST Parser

**TLDR:**

Solidity compiler toolchain with three components:

1. `libs/compiler`

- rust wrapper around Foundry's compiler API with NAPI-generated TypeScript bindings for Node.js/Bun.

2. `libs/shadow`

- zig business logic wrapping Solidity's C++ parser (from ethereum/solidity);
- compiled to WASM via Emscripten, which is required because the C++ parser throws exceptions which needs Emscripten's JavaScript runtime and support—wasm32-freestanding and wasm32-wasi lack C++ exception handling;
- currently we pre-build for a specific solc version so it can't be set dynamically.

3. `libs/shadow-ts`

- TypeScript API consuming the WASM module and Emscripten's auto-generated JavaScript glue code + TypeScript types.

---

Zig-based tool for parsing Solidity code fragments and stitching them into existing contracts with full semantic analysis.

## Quick Start

```bash
zig build test    # Run full test suite
zig build         # Build native library
zig build wasm    # Build WASM module
```

**Requirements:** Zig 0.15+, C++ compiler, Boost libraries, Emscripten, ccache

**Note:** ccache is required for fast incremental WASM builds. Install with `brew install ccache` (macOS) or your system's package manager.

## What It Does

Shadow separates Solidity compilation into two phases:

1. **Parse** - Pure syntax analysis (no semantic checks)
2. **Analyze** - Full semantic validation (13-step pipeline)

This enables:

- Parse Solidity fragments without full contracts
- Stitch parsed functions into existing contracts
- Manipulate ASTs at JSON level
- Run semantic analysis on stitched code

## Project Structure

```
libs/
├── shadow/                       # Shadow parser (Zig)
│   ├── src/
│   │   ├── shadow.zig            # Core parser logic
│   │   ├── utils.zig             # AST utilities
│   │   └── solidity-parser-wrapper.{h,cpp}  # C++ FFI
│   ├── api.zig                   # Native API
│   ├── api_wasm.zig              # WASM API
│   ├── api_emscripten.cpp     # Emscripten bindings
│   └── test/
│       ├── root.zig
│       └── shadow_test.zig       # 30 tests
│
└── compiler/                     # Compiler package (publishable)
    ├── src/lib.rs                # Rust/NAPI source
    ├── build/                    # Generated bindings (committed)
    │   ├── index.js              # Auto-generated loader
    │   ├── index.d.ts            # Auto-generated types
    │   ├── compiler.*.node       # Native binaries
    │   └── npm/                  # Platform packages
    ├── target/                   # Cargo build artifacts (gitignored)
    ├── package.json              # Points to ./build/index.js
    ├── project.json              # Nx build config
    └── test/                     # Tests

dist/wasm/                        # WASM outputs (gitignored)
    ├── shadow.js
    ├── shadow.wasm
    └── shadow.{ts,d.ts}
```

## API Example

```zig
const Shadow = @import("shadow").Shadow;

// Parse shadow function
var shadow = try Shadow.init(allocator,
    "function exploit() public view returns (uint) { return secretValue * 2; }"
);
defer shadow.deinit();

// Stitch into contract
const contract = "contract Vault { uint private secretValue; }";
const analyzed_ast = try shadow.stitchIntoSource(contract, null);
defer allocator.free(analyzed_ast);
```

See [SHADOW_README.md](./SHADOW_README.md) for complete API documentation.

## Build Commands

```bash
zig build           # Native library
zig build test      # Run tests
zig build wasm      # WASM module
zig build typescript  # TypeScript bindings
zig build all       # Everything
zig build clean     # Clean artifacts
```

## Use Cases

- IDE features (syntax highlighting without compilation)
- AST manipulation before semantic validation
- Security analysis (inject test functions)
- Code analysis tools
- WASM-based Solidity parser for browsers

## Technical Details

**C++ Wrapper:** Clean FFI with 4 functions - create/destroy context, parse, analyze

**Custom Analysis Pipeline:** Runs only the 13 semantic analysis steps from CompilerStack (no codegen/optimization)

**JSON Manipulation:** Zig handles AST stitching (ID renumbering, node appending) before passing to C++ analyzer

## Contributing

Key areas:

1. Optimize JSON manipulation
2. WASM improvements
3. Documentation & examples

## License

GPL-3.0 (same as Solidity)

## Documentation

- [SHADOW_README.md](./SHADOW_README.md) - Complete API reference
- [src/test/shadow_test.zig](./src/test/shadow_test.zig) - Test examples
