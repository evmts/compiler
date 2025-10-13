const std = @import("std");
const c = @cImport({
    @cInclude("solidity-parser-wrapper.h");
});

/// Shadow wraps inline Solidity function definitions and parses them into ASTs
/// without requiring them to be semantically valid or compilable.
///
/// This demonstrates that the Solidity parser itself is a pure syntax parser
/// that doesn't perform semantic analysis (like checking if variables are defined).
pub const Shadow = struct {
    allocator: std.mem.Allocator,
    function_source: []const u8,
    ctx: *c.SolParserContext,

    const Self = @This();

    /// Initialize a new Shadow with a function definition string
    pub fn init(allocator: std.mem.Allocator, function_source: []const u8) !Self {
        const ctx = c.sol_parser_create() orelse return error.ParserInitFailed;

        return Self{
            .allocator = allocator,
            .function_source = function_source,
            .ctx = ctx,
        };
    }

    pub fn deinit(self: *Self) void {
        c.sol_parser_destroy(self.ctx);
    }

    /// Wrap the function in minimal boilerplate to make it parseable
    fn wrapFunction(self: *Self, allocator: std.mem.Allocator) ![]u8 {
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
            .{self.function_source},
        );
        return wrapped;
    }

    /// Parse the function into an AST
    /// Returns JSON representation of the AST
    /// This works even if the function has undefined variables!
    pub fn parseToAST(self: *Self) ![]const u8 {
        const wrapped = try self.wrapFunction(self.allocator);
        defer self.allocator.free(wrapped);

        // Add null terminator for C
        const wrapped_cstr = try self.allocator.dupeZ(u8, wrapped);
        defer self.allocator.free(wrapped_cstr);

        const source_name = "Shadow.sol";
        const result = c.sol_parser_parse(
            self.ctx,
            wrapped_cstr.ptr,
            source_name.ptr,
        );

        if (result == null) {
            // Check for errors
            const errors = c.sol_parser_get_errors(self.ctx);
            if (errors != null) {
                defer c.sol_free_string(errors);
                const error_str = std.mem.span(errors);
                std.debug.print("Parser errors:\n{s}\n", .{error_str});
            }
            return error.ParseFailed;
        }

        // Copy the result since we need to free it
        const result_str = std.mem.span(result);
        const owned = try self.allocator.dupe(u8, result_str);
        c.sol_free_string(result);

        return owned;
    }

    /// Parse and extract just the function node from the full contract AST
    pub fn extractFunctionAST(self: *Self) !std.json.Value {
        const ast_json = try self.parseToAST();
        defer self.allocator.free(ast_json);

        var parser = std.json.Parser.init(self.allocator, .alloc_always);
        defer parser.deinit();

        var tree = try parser.parse(ast_json);
        defer tree.deinit();

        // Navigate to the function definition node
        // Structure: root -> nodes[1] (contract) -> nodes[0] (function)
        if (tree.root.object.get("nodes")) |nodes_value| {
            if (nodes_value.array.items.len > 1) {
                const contract = nodes_value.array.items[1];
                if (contract.object.get("nodes")) |contract_nodes| {
                    if (contract_nodes.array.items.len > 0) {
                        return contract_nodes.array.items[0];
                    }
                }
            }
        }

        return error.FunctionNotFound;
    }

    /// Parse a full contract (not just a function)
    pub fn parseFullContract(allocator: std.mem.Allocator, contract_source: []const u8) ![]const u8 {
        const ctx = c.sol_parser_create() orelse return error.ParserInitFailed;
        defer c.sol_parser_destroy(ctx);

        const source_cstr = try allocator.dupeZ(u8, contract_source);
        defer allocator.free(source_cstr);

        const name = "Contract.sol";
        const result = c.sol_parser_parse(ctx, source_cstr.ptr, name.ptr);

        if (result == null) {
            const errors = c.sol_parser_get_errors(ctx);
            if (errors != null) {
                defer c.sol_free_string(errors);
                std.debug.print("Parse errors:\n{s}\n", .{std.mem.span(errors)});
            }
            return error.ParseFailed;
        }

        const result_str = std.mem.span(result);
        const owned = try allocator.dupe(u8, result_str);
        c.sol_free_string(result);
        return owned;
    }

    /// Stitch a shadow function into an existing contract's AST
    pub fn stitchIntoContract(
        allocator: std.mem.Allocator,
        original_contract_ast: []const u8,
        shadow_function_ast: []const u8,
    ) ![]const u8 {
        var original_parser = std.json.Parser.init(allocator, .alloc_always);
        defer original_parser.deinit();

        var shadow_parser = std.json.Parser.init(allocator, .alloc_always);
        defer shadow_parser.deinit();

        var original_tree = try original_parser.parse(original_contract_ast);
        defer original_tree.deinit();

        var shadow_tree = try shadow_parser.parse(shadow_function_ast);
        defer shadow_tree.deinit();

        // Extract the shadow function node
        const shadow_function_node = blk: {
            if (shadow_tree.root.object.get("nodes")) |nodes| {
                if (nodes.array.items.len > 1) {
                    const contract = nodes.array.items[1];
                    if (contract.object.get("nodes")) |contract_nodes| {
                        if (contract_nodes.array.items.len > 0) {
                            break :blk contract_nodes.array.items[0];
                        }
                    }
                }
            }
            return error.ShadowFunctionNotFound;
        };

        // Get the contract node from original
        var contract_node = blk: {
            if (original_tree.root.object.get("nodes")) |nodes| {
                if (nodes.array.items.len > 1) {
                    break :blk &nodes.array.items[1];
                }
            }
            return error.OriginalContractNotFound;
        };

        // Add shadow function to contract's nodes
        if (contract_node.object.getPtr("nodes")) |contract_nodes_ptr| {
            try contract_nodes_ptr.array.append(allocator, shadow_function_node);
        } else {
            return error.ContractNodesNotFound;
        }

        // Serialize back to JSON
        var output = std.ArrayList(u8).init(allocator);
        defer output.deinit();

        try std.json.stringify(original_tree.root, .{}, output.writer());
        return output.toOwnedSlice();
    }
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    std.debug.print("=== Shadow Parser Test ===\n\n", .{});

    // Test 1: Basic parsing tests
    std.debug.print("Test 1: Function with undefined variable\n", .{});
    const bad_function =
        \\function badFunction() public returns (uint) {
        \\    return undefinedVariable + 5;
        \\}
    ;

    var shadow1 = try Shadow.init(allocator, bad_function);
    defer shadow1.deinit();

    const ast1 = shadow1.parseToAST() catch |err| {
        std.debug.print("Parse failed: {}\n", .{err});
        return;
    };
    defer allocator.free(ast1);

    std.debug.print("✓ Successfully parsed function with undefined variable!\n\n", .{});

    // Test 2: AST Stitching Demo
    std.debug.print("=== AST Stitching Demo ===\n\n", .{});

    // Original valid contract with private state
    const original_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract MyContract {
        \\    uint private secretValue;
        \\    address private owner;
        \\
        \\    constructor() {
        \\        owner = msg.sender;
        \\        secretValue = 42;
        \\    }
        \\
        \\    function getOwner() public view returns (address) {
        \\        return owner;
        \\    }
        \\}
    ;

    // Shadow function that accesses private variables
    const shadow_function =
        \\function shadowExploit() public view returns (uint) {
        \\    return secretValue + 100;
        \\}
    ;

    std.debug.print("Step 1: Parse original valid contract\n", .{});
    const original_ast = try Shadow.parseFullContract(allocator, original_contract);
    defer allocator.free(original_ast);
    std.debug.print("✓ Original contract parsed ({} bytes)\n\n", .{original_ast.len});

    std.debug.print("Step 2: Parse shadow function (references private secretValue)\n", .{});
    var shadow = try Shadow.init(allocator, shadow_function);
    defer shadow.deinit();
    const shadow_ast = try shadow.parseToAST();
    defer allocator.free(shadow_ast);
    std.debug.print("✓ Shadow function parsed ({} bytes)\n", .{shadow_ast.len});
    std.debug.print("   Note: Parser doesn't care that secretValue isn't defined!\n\n", .{});

    std.debug.print("Step 3: Stitch shadow function into original contract's AST\n", .{});
    const stitched_ast = try Shadow.stitchIntoContract(allocator, original_ast, shadow_ast);
    defer allocator.free(stitched_ast);
    std.debug.print("✓ ASTs stitched together ({} bytes)\n\n", .{stitched_ast.len});

    // Verify the stitched AST contains both
    const has_get_owner = std.mem.indexOf(u8, stitched_ast, "getOwner") != null;
    const has_shadow = std.mem.indexOf(u8, stitched_ast, "shadowExploit") != null;
    const has_secret = std.mem.indexOf(u8, stitched_ast, "secretValue") != null;

    std.debug.print("Verification:\n", .{});
    std.debug.print("  - Contains getOwner(): {}\n", .{has_get_owner});
    std.debug.print("  - Contains shadowExploit(): {}\n", .{has_shadow});
    std.debug.print("  - References secretValue: {}\n", .{has_secret});

    if (has_get_owner and has_shadow and has_secret) {
        std.debug.print("\n✓ SUCCESS: Shadow function successfully stitched!\n", .{});
        std.debug.print("  The combined AST contains:\n", .{});
        std.debug.print("  - Original: private variable 'secretValue'\n", .{});
        std.debug.print("  - Original: function 'getOwner()'\n", .{});
        std.debug.print("  - Shadow: function 'shadowExploit()' accessing secretValue\n", .{});
        std.debug.print("\n  Parser doesn't validate access control!\n", .{});

        // Save the stitched AST
        const file = try std.fs.cwd().createFile("stitched_ast.json", .{});
        defer file.close();
        try file.writeAll(stitched_ast);
        std.debug.print("\n✓ Saved stitched AST to 'stitched_ast.json'\n", .{});
    }

    std.debug.print("\n=== All demos completed! ===\n", .{});
}
