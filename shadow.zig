const std = @import("std");
const c = @cImport({
    @cInclude("solidity-parser-wrapper.h");
});

/// Shadow wraps Solidity code fragments (functions, variables, etc.) and parses
/// them into ASTs without requiring semantic validity.
///
/// Demonstrates that the Solidity parser performs pure syntax analysis without
/// semantic validation (e.g., doesn't check if variables are defined).
pub const Shadow = struct {
    allocator: std.mem.Allocator,
    source: []const u8,
    ctx: *c.SolParserContext,

    const Self = @This();

    pub const Error = error{
        /// C parser context creation failed
        ParserInitFailed,
        /// Syntax error during parsing
        ParseFailed,
        /// AST contains no extractable nodes
        NoNodesFound,
        /// Target AST has unexpected structure
        InvalidContractStructure,
        /// Memory allocation failed
        OutOfMemory,
        /// JSON parsing errors
        Overflow,
        InvalidCharacter,
        UnexpectedToken,
        InvalidNumber,
        InvalidEnumTag,
        DuplicateField,
        UnknownField,
        MissingField,
        LengthMismatch,
        SyntaxError,
        UnexpectedEndOfInput,
        BufferUnderrun,
        ValueTooLong,
    };

    /// Initialize a new Shadow with a function definition string
    pub fn init(allocator: std.mem.Allocator, source: []const u8) Error!Self {
        const ctx = c.sol_parser_create() orelse return Error.ParserInitFailed;
        return Self{
            .allocator = allocator,
            .source = source,
            .ctx = ctx,
        };
    }

    pub fn deinit(self: *Self) void {
        c.sol_parser_destroy(self.ctx);
    }

    /// Wrap the shadow source in minimal boilerplate to make it parseable
    fn wrapShadowSource(self: *Self, allocator: std.mem.Allocator) ![]u8 {
        // Create minimal boilerplate that makes it syntactically valid
        // Note: This code won't COMPILE due to semantic errors, but it will PARSE!
        const wrapped = try std.fmt.allocPrint(
            allocator,
            \\// SPDX-License-Identifier: UNLICENSED
            \\pragma solidity ^0.8.0;
            \\
            \\contract Shadow {{
            \\    {s}
            \\}}
            \\
        ,
            .{self.source},
        );
        return wrapped;
    }

    /// Parse the function into an AST
    /// Returns JSON representation of the AST
    /// This works even if the function has undefined variables!
    pub fn toWrappedAst(self: *Self) Error![]const u8 {
        const wrapped = try self.wrapShadowSource(self.allocator);
        defer self.allocator.free(wrapped);
        return try parseSourceAst(self.allocator, wrapped, "Shadow.sol");
    }

    /// Parse and extract all function nodes from the full contract AST as JSON strings
    pub fn toAstNodes(self: *Self) Error![][]const u8 {
        const ast_json = try self.toWrappedAst();
        defer self.allocator.free(ast_json);

        var parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, ast_json, .{});
        defer parsed.deinit();

        // Navigate to contract nodes
        // Structure: root -> nodes[1] (contract) -> nodes[...] (functions)
        if (parsed.value.object.get("nodes")) |nodes_value| {
            if (nodes_value.array.items.len > 1) {
                const contract = nodes_value.array.items[1];
                if (contract.object.get("nodes")) |contract_nodes| {
                    const node_count = contract_nodes.array.items.len;
                    if (node_count == 0) return Error.NoNodesFound;

                    var nodes = try self.allocator.alloc([]const u8, node_count);
                    for (contract_nodes.array.items, 0..) |node, i| {
                        nodes[i] = try std.fmt.allocPrint(self.allocator, "{f}", .{std.json.fmt(node, .{})});
                    }
                    return nodes;
                }
            }
        }

        return Error.NoNodesFound;
    }

    /// Parse complete Solidity source code to AST.
    /// Used internally by stitchIntoSource(), but also available for general use.
    pub fn parseSourceAst(allocator: std.mem.Allocator, source: []const u8, name: ?[]const u8) Error![]const u8 {
        const ctx = c.sol_parser_create() orelse return Error.ParserInitFailed;
        defer c.sol_parser_destroy(ctx);

        const source_cstr = try allocator.dupeZ(u8, source);
        defer allocator.free(source_cstr);

        const source_name = name orelse "Contract.sol";
        const result = c.sol_parser_parse(ctx, source_cstr.ptr, source_name.ptr);

        if (result == null) {
            const errors = c.sol_parser_get_errors(ctx);
            if (errors != null) {
                defer c.sol_free_string(errors);
                std.debug.print("Parse errors:\n{s}\n", .{std.mem.span(errors)});
            }
            return Error.ParseFailed;
        }

        const result_str = std.mem.span(result);
        const owned = try allocator.dupe(u8, result_str);
        c.sol_free_string(result);
        return owned;
    }

    /// Stitch shadow function(s) into an existing contract's AST
    /// Handles both single and multiple functions from shadow AST
    pub fn stitchIntoAst(self: *Self, target_ast: []const u8) Error![]const u8 {
        var target_parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, target_ast, .{});
        defer target_parsed.deinit();

        const shadow_ast = try self.toWrappedAst();
        defer self.allocator.free(shadow_ast);

        var shadow_parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, shadow_ast, .{});
        defer shadow_parsed.deinit();

        // Extract all shadow nodes
        const shadow_nodes = blk: {
            if (shadow_parsed.value.object.get("nodes")) |root_nodes| {
                if (root_nodes.array.items.len > 1) {
                    const contract = root_nodes.array.items[1];
                    if (contract.object.get("nodes")) |nodes| {
                        if (nodes.array.items.len > 0) {
                            break :blk nodes.array.items;
                        }
                    }
                }
            }
            return Error.NoNodesFound;
        };

        // Get the contract node from target
        var target_contract = blk: {
            if (target_parsed.value.object.get("nodes")) |root_nodes| {
                if (root_nodes.array.items.len > 1) {
                    break :blk &root_nodes.array.items[1];
                }
            }
            return Error.InvalidContractStructure;
        };

        // Add all shadow nodes to contract's nodes
        if (target_contract.object.getPtr("nodes")) |target_nodes_ptr| {
            for (shadow_nodes) |node| {
                try target_nodes_ptr.array.append(node);
            }
        } else {
            return Error.InvalidContractStructure;
        }

        // Serialize back to JSON
        return try std.fmt.allocPrint(self.allocator, "{f}", .{std.json.fmt(target_parsed.value, .{})});
    }

    /// Stitch shadow function(s) into an existing contract's source code
    /// Convenience wrapper that parses the source first
    pub fn stitchIntoSource(self: *Self, target_source: []const u8) Error![]const u8 {
        const target_ast = try parseSourceAst(self.allocator, target_source, null);
        defer self.allocator.free(target_ast);
        return try self.stitchIntoAst(target_ast);
    }
};

// ============================================================================
// Tests
// ============================================================================

test "parse function with undefined variable" {
    const allocator = std.testing.allocator;

    const bad_function =
        \\function badFunction() public returns (uint) {
        \\    return undefinedVariable + 5;
        \\}
    ;

    var shadow = try Shadow.init(allocator, bad_function);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "badFunction") != null);
}

test "parse function with type mismatch" {
    const allocator = std.testing.allocator;

    const type_error_function =
        \\function typeErrorFunction() public {
        \\    uint x = "this is a string not a uint";
        \\}
    ;

    var shadow = try Shadow.init(allocator, type_error_function);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "typeErrorFunction") != null);
}

test "parse function calling non-existent function" {
    const allocator = std.testing.allocator;

    const missing_func =
        \\function callerFunction() public {
        \\    nonExistentFunction();
        \\}
    ;

    var shadow = try Shadow.init(allocator, missing_func);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "callerFunction") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "nonExistentFunction") != null);
}

test "parse function with multiple undefined variables" {
    const allocator = std.testing.allocator;

    const multi_undefined =
        \\function complexFunction(uint a) public returns (uint) {
        \\    uint result = a + undefinedVar1;
        \\    result = result * undefinedVar2;
        \\    return result + undefinedVar3;
        \\}
    ;

    var shadow = try Shadow.init(allocator, multi_undefined);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "complexFunction") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "undefinedVar1") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "undefinedVar2") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "undefinedVar3") != null);
}

test "parse function with invalid struct access" {
    const allocator = std.testing.allocator;

    const invalid_struct =
        \\function accessFunction() public {
        \\    NonExistentStruct memory obj;
        \\    obj.someField = 42;
        \\}
    ;

    var shadow = try Shadow.init(allocator, invalid_struct);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "accessFunction") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "NonExistentStruct") != null);
}

test "parse valid function for comparison" {
    const allocator = std.testing.allocator;

    const valid_function =
        \\function validFunction(uint x) public pure returns (uint) {
        \\    return x + 10;
        \\}
    ;

    var shadow = try Shadow.init(allocator, valid_function);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "validFunction") != null);
}

test "parse function with syntax error should fail" {
    const allocator = std.testing.allocator;

    const syntax_error =
        \\function syntaxError() public {
        \\    return 42;
    ;

    var shadow = try Shadow.init(allocator, syntax_error);
    defer shadow.deinit();

    const result = shadow.toWrappedAst();
    try std.testing.expectError(Shadow.Error.ParseFailed, result);
}

test "Shadow init and deinit" {
    const allocator = std.testing.allocator;

    const simple_func = "function test() public {}";

    var shadow = try Shadow.init(allocator, simple_func);
    shadow.deinit();

    try std.testing.expect(true);
}

test "stitch shadow function into valid contract" {
    const allocator = std.testing.allocator;

    const original_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract MyContract {
        \\    uint private secretValue;
        \\
        \\    function getSecret() public view returns (uint) {
        \\        return secretValue;
        \\    }
        \\}
    ;

    const shadow_function =
        \\function exploit() public view returns (uint) {
        \\    return secretValue * 2;
        \\}
    ;

    const original_ast = try Shadow.parseSourceAst(allocator, original_contract, null);
    defer allocator.free(original_ast);

    var shadow = try Shadow.init(allocator, shadow_function);
    defer shadow.deinit();

    const stitched_ast = try shadow.stitchIntoAst(original_ast);
    defer allocator.free(stitched_ast);

    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "getSecret") != null);
    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "exploit") != null);
    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "secretValue") != null);
}

test "parse full contract directly" {
    const allocator = std.testing.allocator;

    const contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract Test {
        \\    function foo() public {}
        \\}
    ;

    const ast = try Shadow.parseSourceAst(allocator, contract, null);
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "Test") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "foo") != null);
}

test "parse multiple valid functions" {
    const allocator = std.testing.allocator;

    const multi_functions =
        \\function first(uint x) public pure returns (uint) {
        \\    return x + 1;
        \\}
        \\
        \\function second(uint y) public pure returns (uint) {
        \\    return y * 2;
        \\}
        \\
        \\function third() public pure returns (uint) {
        \\    return 42;
        \\}
    ;

    var shadow = try Shadow.init(allocator, multi_functions);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(std.mem.indexOf(u8, ast, "first") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "second") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "third") != null);
}

test "parse multiple functions with semantic errors" {
    const allocator = std.testing.allocator;

    const bad_functions =
        \\function useUndefined() public returns (uint) {
        \\    return undefinedVar;
        \\}
        \\
        \\function typeError() public {
        \\    uint x = "not a number";
        \\}
        \\
        \\function callMissing() public {
        \\    nonExistentFunction();
        \\}
    ;

    var shadow = try Shadow.init(allocator, bad_functions);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(std.mem.indexOf(u8, ast, "useUndefined") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "typeError") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "callMissing") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "undefinedVar") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "nonExistentFunction") != null);
}

test "extractAllFunctionASTs returns all functions" {
    const allocator = std.testing.allocator;

    const multi_functions =
        \\function alpha() public {}
        \\function beta() public {}
        \\function gamma() public {}
    ;

    var shadow = try Shadow.init(allocator, multi_functions);
    defer shadow.deinit();

    const functions = try shadow.toAstNodes();
    defer {
        for (functions) |func| {
            allocator.free(func);
        }
        allocator.free(functions);
    }

    try std.testing.expectEqual(@as(usize, 3), functions.len);

    for (functions) |func_json| {
        try std.testing.expect(std.mem.indexOf(u8, func_json, "nodeType") != null);
    }
}

test "extractAllFunctionASTs with mixed semantic validity" {
    const allocator = std.testing.allocator;

    const mixed_functions =
        \\function valid(uint x) public pure returns (uint) {
        \\    return x;
        \\}
        \\
        \\function invalid() public returns (uint) {
        \\    return missingVariable + 10;
        \\}
    ;

    var shadow = try Shadow.init(allocator, mixed_functions);
    defer shadow.deinit();

    const functions = try shadow.toAstNodes();
    defer {
        for (functions) |func| {
            allocator.free(func);
        }
        allocator.free(functions);
    }

    try std.testing.expectEqual(@as(usize, 2), functions.len);
}

test "stitch multiple shadow functions into contract" {
    const allocator = std.testing.allocator;

    const original_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract MyContract {
        \\    uint private data;
        \\
        \\    function getData() public view returns (uint) {
        \\        return data;
        \\    }
        \\}
    ;

    const shadow_functions =
        \\function exploitOne() public view returns (uint) {
        \\    return data * 2;
        \\}
        \\
        \\function exploitTwo() public view returns (uint) {
        \\    return data + 100;
        \\}
    ;

    const original_ast = try Shadow.parseSourceAst(allocator, original_contract, null);
    defer allocator.free(original_ast);

    var shadow = try Shadow.init(allocator, shadow_functions);
    defer shadow.deinit();

    const stitched_ast = try shadow.stitchIntoAst(original_ast);
    defer allocator.free(stitched_ast);

    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "getData") != null);
    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "exploitOne") != null);
    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "exploitTwo") != null);
    try std.testing.expect(std.mem.indexOf(u8, stitched_ast, "data") != null);
}

test "single line function should parse" {
    const allocator = std.testing.allocator;

    const single_line = "function test() public {}";

    var shadow = try Shadow.init(allocator, single_line);
    defer shadow.deinit();

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "test") != null);
}

test "complex multi-function with state variables access" {
    const allocator = std.testing.allocator;

    const complex_functions =
        \\function accessPrivate() public view returns (uint) {
        \\    return secretValue;
        \\}
        \\
        \\function modifyPrivate(uint newValue) public {
        \\    secretValue = newValue;
        \\}
        \\
        \\function computePrivate() public view returns (uint) {
        \\    return secretValue * otherSecret;
        \\}
    ;

    var shadow = try Shadow.init(allocator, complex_functions);
    defer shadow.deinit();

    const functions = try shadow.toAstNodes();
    defer {
        for (functions) |func| {
            allocator.free(func);
        }
        allocator.free(functions);
    }

    try std.testing.expectEqual(@as(usize, 3), functions.len);

    const ast = try shadow.toWrappedAst();
    defer allocator.free(ast);

    try std.testing.expect(std.mem.indexOf(u8, ast, "accessPrivate") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "modifyPrivate") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "computePrivate") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "secretValue") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "otherSecret") != null);
}
