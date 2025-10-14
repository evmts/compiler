# Shadow - API Reference

Complete API documentation for Shadow, a Zig library for parsing and stitching Solidity code fragments.

**Two-phase operation:**
1. **Parse** - Syntax analysis only
2. **Analyze** - Full semantic validation (when stitching)

## API Reference

### Shadow Struct

```zig
const Shadow = @import("shadow").Shadow;

pub const Shadow = struct {
    allocator: std.mem.Allocator,
    source: []const u8,
    ctx: *c.SolParserContext,

    // ... methods below
}
```

### Initialization

#### `init(allocator: Allocator, source: []const u8) !Shadow`

Creates a new Shadow instance with Solidity code fragment.

**Parameters:**
- `allocator` - Memory allocator
- `source` - Solidity code (function, variable, etc.)

**Example:**
```zig
const allocator = std.heap.page_allocator;

var shadow = try Shadow.init(allocator,
    \\function exploit() public view returns (uint) {
    \\    return secretValue * 2;
    \\}
);
defer shadow.deinit();
```

#### `deinit(self: *Self) void`

Cleans up parser context. Always call after use.

### Parsing Functions

#### `parseSourceAst(allocator: Allocator, source: []const u8, name: ?[]const u8) ![]const u8`

Static method to parse complete Solidity source to AST JSON.

**Parameters:**
- `allocator` - Memory allocator
- `source` - Complete Solidity source code
- `name` - Optional source name (default: "Contract.sol")

**Returns:** JSON string of parsed AST (caller must free)

**Example:**
```zig
const contract =
    \\// SPDX-License-Identifier: MIT
    \\pragma solidity ^0.8.0;
    \\contract MyContract {
    \\    uint private value;
    \\    function getValue() public view returns (uint) {
    \\        return value;
    \\    }
    \\}
;

const ast_json = try Shadow.parseSourceAst(allocator, contract, null);
defer allocator.free(ast_json);

// ast_json is JSON representation of parsed AST
```

#### `toAstNodes(self: *Self) ![][]const u8`

Extracts individual AST nodes from shadow contract.

**Returns:** Array of JSON strings, one per node (caller must free all)

**Example:**
```zig
const multi_functions =
    \\function alpha() public {}
    \\function beta() public view returns (uint) { return 42; }
    \\function gamma() external payable {}
;

var shadow = try Shadow.init(allocator, multi_functions);
defer shadow.deinit();

const nodes = try shadow.toAstNodes();
defer {
    for (nodes) |node| allocator.free(node);
    allocator.free(nodes);
}

// nodes[0] = JSON for alpha function
// nodes[1] = JSON for beta function
// nodes[2] = JSON for gamma function
```

### Stitching Functions

#### `stitchIntoAst(self: *Self, target_ast: []const u8) ![]const u8`

Stitches shadow function(s) into existing contract AST.

**Process:**
1. Parse shadow code (wrapped in minimal boilerplate)
2. Parse target AST JSON
3. Find max ID in target (avoid collisions)
4. Renumber all shadow node IDs (offset by max)
5. Append shadow nodes to target contract
6. Serialize combined JSON
7. Pass to C++ analyzer (13-step semantic analysis)
8. Return fully analyzed AST with semantic annotations

**Parameters:**
- `target_ast` - JSON AST from `parseSourceAst()`

**Returns:** Fully analyzed AST JSON with semantic annotations (caller must free)

**Example:**
```zig
const target_contract =
    \\// SPDX-License-Identifier: MIT
    \\pragma solidity ^0.8.0;
    \\contract Vault {
    \\    uint private balance;
    \\    function getBalance() public view returns (uint) {
    \\        return balance;
    \\    }
    \\}
;

const shadow_func =
    \\function stealBalance() public view returns (uint) {
    \\    return balance;
    \\}
;

// Parse target to AST
const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
defer allocator.free(target_ast);

// Create shadow and stitch
var shadow = try Shadow.init(allocator, shadow_func);
defer shadow.deinit();

const analyzed_ast = try shadow.stitchIntoAst(target_ast);
defer allocator.free(analyzed_ast);

// analyzed_ast contains both getBalance() and stealBalance()
// with full semantic annotations (types, references, etc.)
```

#### `stitchIntoSource(self: *Self, target_source: []const u8, name: ?[]const u8) ![]const u8`

Convenience wrapper - parses source then stitches.

**Parameters:**
- `target_source` - Solidity source code
- `name` - Optional source name

**Returns:** Fully analyzed AST JSON (caller must free)

**Example:**
```zig
const target_source =
    \\// SPDX-License-Identifier: MIT
    \\pragma solidity ^0.8.0;
    \\contract Token {
    \\    mapping(address => uint) private balances;
    \\}
;

const shadow_func = "function getBalance(address a) public view returns (uint) { return balances[a]; }";

var shadow = try Shadow.init(allocator, shadow_func);
defer shadow.deinit();

const analyzed_ast = try shadow.stitchIntoSource(target_source, null);
defer allocator.free(analyzed_ast);
```

## How Stitching Works

1. **Parse** - Both target and shadow parsed independently (syntax only)
2. **Manipulate** - Zig finds max ID in target, renumbers shadow IDs, appends shadow nodes
3. **Analyze** - C++ imports stitched JSON, runs 13-step semantic validation, exports with annotations

**Semantic annotations added:**
- `typeDescriptions` - Type info for expressions
- `referencedDeclaration` - Definition IDs
- `scope` - Scope information

## Error Handling

All functions return Zig errors:

```zig
pub const Error = error{
    ParserInitFailed,    // C parser context creation failed
    ParseFailed,         // Syntax error during parsing
    AnalysisFailed,      // Semantic analysis failed
    NoNodesFound,        // AST contains no extractable nodes
    InvalidContractStructure,  // Target AST unexpected structure
    OutOfMemory,         // Memory allocation failed
    // ... JSON parsing errors
};
```

**Example error handling:**
```zig
const result = shadow.stitchIntoSource(source, null);
if (result) |ast| {
    defer allocator.free(ast);
    // Use ast
} else |err| switch (err) {
    error.ParseFailed => {
        // Get detailed errors
        const errors = c.sol_parser_get_errors(shadow.ctx);
        if (errors != null) {
            defer c.sol_free_string(errors);
            std.debug.print("Parse errors: {s}\n", .{std.mem.span(errors)});
        }
    },
    error.AnalysisFailed => {
        std.debug.print("Analysis failed - semantic errors\n", .{});
    },
    else => {},
}
```

## Known Limitations

The following features work:
- ✅ **Structs** - Custom struct type resolution
- ✅ **Events** - Event emission in shadow functions
- ✅ **Custom Errors** - Revert with custom error types

The following features don't work yet (failing tests):
- ❌ **Inheritance** - Multi-contract sources with `is` keyword
- ❌ **Interfaces** - Interface + contract in same source
- ❌ **Libraries** - Library resolution and `using` statements

These are documented as failing tests in `src/test/shadow_test.zig` under "Complex Features Tests".

## Examples

### Example 1: Parse Any Solidity Fragment

```zig
const fragments = [_][]const u8{
    "function test() public {}",
    "uint private x;",
    "event Transfer(address indexed from, address to, uint value);",
    "modifier onlyOwner() { require(msg.sender == owner); _; }",
};

for (fragments) |fragment| {
    var shadow = try Shadow.init(allocator, fragment);
    defer shadow.deinit();

    // All parse successfully despite missing context!
    const nodes = try shadow.toAstNodes();
    defer {
        for (nodes) |node| allocator.free(node);
        allocator.free(nodes);
    }
}
```

### Example 2: Security Analysis - Inject Backdoor

```zig
const target =
    \\contract Token {
    \\    mapping(address => uint) balances;
    \\    address owner;
    \\
    \\    modifier onlyOwner() {
    \\        require(msg.sender == owner);
    \\        _;
    \\    }
    \\
    \\    function transfer(address to, uint amount) public {
    \\        require(balances[msg.sender] >= amount);
    \\        balances[msg.sender] -= amount;
    \\        balances[to] += amount;
    \\    }
    \\}
;

// Inject backdoor that bypasses onlyOwner
const backdoor =
    \\function backdoorTransfer(address to, uint amount) public {
    \\    balances[msg.sender] -= amount;
    \\    balances[to] += amount;
    \\}
;

var shadow = try Shadow.init(allocator, backdoor);
defer shadow.deinit();

const exploited = try shadow.stitchIntoSource(target, null);
defer allocator.free(exploited);

// exploited now contains both transfer() and backdoorTransfer()
// Use exploited AST for security analysis, testing, etc.
```

### Example 3: IDE Features - Parse Incomplete Code

```zig
// User is typing in IDE, incomplete function
const incomplete = "function calculateReward(uint stake) public view returns";

var shadow = try Shadow.init(allocator, incomplete);
defer shadow.deinit();

// Try to parse (will fail - syntax error)
const result = shadow.toAstNodes();
if (result) |_| {
    // Provide completion suggestions
} else |_| {
    // Parse failed - show error, but don't block editing
}
```

## Testing

```bash
zig build test  # 30 tests - 27 passing, 3 failing
```

See `src/test/shadow_test.zig` for examples.

## License

GPL-3.0 (same as Solidity)
