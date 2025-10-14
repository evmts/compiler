# @tevm/shadow

TypeScript bindings for the Shadow Solidity parser, compiled to WebAssembly. Provides full TypeScript type safety with auto-generated types from C++ Emscripten bindings.

## Features

- **Full Solidity 0.8.31 parser** - Parse any Solidity source code into AST
- **Contract stitching** - Merge shadow contracts into target contracts (source or AST)
- **Auto-generated types** - TypeScript types generated directly from C++ bindings
- **Memory safe** - Manual disposal pattern prevents WASM memory leaks
- **Fast** - Native performance via WebAssembly

## Installation

```bash
npm install @tevm/shadow
# or
pnpm add @tevm/shadow
# or
bun add @tevm/shadow
```

## Quick Start

```typescript
import { Shadow } from "@tevm/shadow";

// Initialize WASM module (do this once at startup)
await Shadow.init();

// Parse Solidity source to AST
const ast = Shadow.parseSource("contract Foo {}", "Foo.sol");
const parsed = JSON.parse(ast);

// Create shadow contract and stitch into target
const shadow = Shadow.create("contract Shadow {}");
const result = shadow.stitchIntoSource("contract Target {}");
shadow.destroy(); // Clean up WASM memory
```

## Usage

### Initialize the WASM module

The WASM module must be loaded before using any parsing or stitching operations:

```typescript
import { Shadow } from "@tevm/shadow";

// Load once at application startup
await Shadow.init();

// Subsequent calls return the same cached module
const module = await Shadow.init();
```

### Parse Solidity source

The `parseSource` static method parses Solidity source code and returns the AST as JSON:

```typescript
import { Shadow } from "@tevm/shadow";

const source = `
  contract HelloWorld {
    function greet() public pure returns (string memory) {
      return "Hello, World!";
    }
  }
`;

// parseSource is synchronous after Shadow.init()
const ast = Shadow.parseSource(source, "HelloWorld.sol");
const parsed = JSON.parse(ast);

console.log(parsed.nodeType); // "SourceUnit"
console.log(parsed.nodes[0].name); // "HelloWorld"
```

### Use Shadow for contract stitching

```typescript
import { Shadow } from "@tevm/shadow";

// Create a shadow contract
const shadow = Shadow.create(`
  contract ShadowCounter {
    uint256 private _value;

    function increment() internal {
      _value++;
    }
  }
`);

// Stitch into target contract
const target = `
  contract Counter {
    function add() public {
      // Will have shadow functionality added
    }
  }
`;

const result = shadow.stitchIntoSource(target, "Counter.sol", "Counter");
console.log(result);

// Clean up (important to prevent memory leaks!)
shadow.destroy();
```

### Using explicit resource management (TypeScript 5.2+)

```typescript
{
  using shadow = Shadow.create('contract Foo {}');
  const result = shadow.stitchIntoSource('contract Bar {}');
  // shadow is automatically destroyed at end of block
}
```

## API Reference

### Module Loading

#### `Shadow.init(): Promise<MainModule>`

Initializes the WASM module. Must be called before using any Shadow operations.

**Returns:** Promise resolving to the loaded WASM module

**Example:**

```typescript
const module = await Shadow.init();
// Module is now cached, subsequent calls return same instance
```

### Static Methods

#### `Shadow.parseSource(source: string, name?: string): string`

Parse Solidity source code and return the AST as a JSON string. This is a **synchronous** method (throws if WASM not loaded).

**Parameters:**

- `source: string` - Solidity source code to parse
- `name?: string` - Optional file name for the source (defaults to empty string)

**Returns:** JSON string containing the Solidity AST

**Throws:** Error if WASM module not loaded via `Shadow.init()`

**Example:**

```typescript
await Shadow.init();
const ast = Shadow.parseSource(
  `
  contract ERC20 {
    mapping(address => uint256) public balanceOf;
  }
`,
  "ERC20.sol"
);

const parsed = JSON.parse(ast);
console.log(parsed.nodes[0].nodeType); // "ContractDefinition"
```

#### `Shadow.create(source: string): Shadow`

Create a Shadow instance from shadow contract source code.

**Parameters:**

- `source: string` - Shadow contract source code

**Returns:** Shadow instance

**Throws:** Error if source code has syntax errors

**Example:**

```typescript
const shadow = Shadow.create(`
  contract Logger {
    event Log(string message);
    function log(string memory msg) internal {
      emit Log(msg);
    }
  }
`);
```

### Instance Methods

#### `shadow.stitchIntoSource(target: string, sourceName?: string, contractName?: string): string`

Stitch the shadow contract into target source code.

**Parameters:**

- `target: string` - Target Solidity source code
- `sourceName?: string` - Optional source name for the result
- `contractName?: string` - Optional specific contract name to target

**Returns:** Modified source code with shadow contract stitched in

**Throws:** Error if instance destroyed or target has syntax errors

**Example:**

```typescript
const shadow = Shadow.create("contract Shadow {}");
const stitched = shadow.stitchIntoSource("contract Target { uint256 x; }", "Target.sol", "Target");
```

#### `shadow.stitchIntoAst(targetAst: string, contractName?: string): string`

Stitch the shadow contract into a target AST (JSON format).

**Parameters:**

- `targetAst: string` - Target AST as JSON string
- `contractName?: string` - Optional specific contract name to target

**Returns:** Modified AST as JSON string

**Throws:** Error if instance destroyed or AST is invalid

**Example:**

```typescript
const targetAst = Shadow.parseSource("contract Target {}");
const shadow = Shadow.create("contract Shadow {}");
const stitchedAst = shadow.stitchIntoAst(targetAst, "Target");
const result = JSON.parse(stitchedAst);
```

#### `shadow.destroy(): void`

Free WASM memory associated with this Shadow instance. **Always call this** when done to prevent memory leaks.

**Example:**

```typescript
const shadow = Shadow.create("contract Foo {}");
try {
  const result = shadow.stitchIntoSource("contract Bar {}");
} finally {
  shadow.destroy(); // Clean up even if operation fails
}
```

## Architecture

This package provides TypeScript bindings for a WebAssembly module built from multiple languages:

### Build Pipeline

```
┌──────────────┐
│ Solidity     │  Solidity 0.8.31 AST parser
│ C++ Parser   │  (from ethereum/solidity)
└──────┬───────┘
       │
┌──────▼───────┐
│ Zig Business │  Shadow contract stitching logic
│ Logic        │  AST manipulation
└──────┬───────┘
       │
┌──────▼───────┐
│ C++ Bindings │  Emscripten embind exports
│ (api_emscri  │  Exception handling
│ pten.cpp)    │
└──────┬───────┘
       │
┌──────▼───────┐
│ Emscripten   │  Compile to WASM
│              │  --emit-tsd for types
└──────┬───────┘
       │
┌──────▼───────┐
│ WASM Module  │  shadow.wasm + shadow.js
│ + TypeScript │  + shadow.d.ts (auto-generated)
│ Types        │
└──────────────┘
```

### Type Safety

TypeScript types are **automatically generated** from C++ Emscripten bindings using `--emit-tsd`:

- `wasm/shadow.d.ts` - Auto-generated from C++ API
- Single source of truth: C++ bindings
- Type safety guaranteed to match WASM API
- No manual type maintenance required

### Components

1. **Solidity Parser (C++)** - Full Solidity 0.8.31 parser from ethereum/solidity
2. **Shadow Logic (Zig)** - Contract stitching and AST manipulation
3. **C++ Bindings** - Emscripten embind exports with exception handling
4. **WASM Module** - Compiled with wasm32-wasi target
5. **TypeScript Wrapper** - High-level API with memory management

### Memory Management

Shadow instances hold WASM memory and must be manually destroyed:

```typescript
const shadow = Shadow.create("contract Foo {}");
shadow.stitchIntoSource("contract Bar {}");
shadow.destroy(); // Free WASM memory
```

Alternatively, use explicit resource management (TS 5.2+):

```typescript
{
  using shadow = Shadow.create('contract Foo {}');
  // Automatically destroyed at block end
}
```

## Development

### Building from Source

```bash
# Build WASM module (requires Zig + Emscripten)
zig build wasm

# Build TypeScript bindings
pnpm build

# Run tests
bun test
```

### Project Structure

```
libs/shadow-ts/
├── src/
│   ├── index.ts          # TypeScript wrapper API
│   └── index.test.ts     # Bun tests
├── wasm/
│   ├── shadow.js         # Emscripten loader (auto-generated)
│   ├── shadow.wasm       # WASM binary (auto-generated)
│   └── shadow.d.ts       # TypeScript types (auto-generated)
├── dist/                 # Built TypeScript output
├── package.json
├── tsconfig.json
└── README.md
```

## License

MIT
