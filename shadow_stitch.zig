const std = @import("std");
const c = @cImport({
    @cInclude("solidity-parser-wrapper.h");
});

/// ShadowStitcher demonstrates parsing a valid contract and a shadow function separately,
/// then combining their ASTs. The shadow function can reference private variables from
/// the target contract without the parser complaining.
pub const ShadowStitcher = struct {
    allocator: std.mem.Allocator,
    ctx: *c.SolParserContext,

    const Self = @This();

    pub fn init(allocator: std.mem.Allocator) !Self {
        const ctx = c.sol_parser_create() orelse return error.ParserInitFailed;
        return Self{
            .allocator = allocator,
            .ctx = ctx,
        };
    }

    pub fn deinit(self: *Self) void {
        c.sol_parser_destroy(self.ctx);
    }

    /// Parse a complete valid contract
    pub fn parseContract(self: *Self, contract_source: []const u8, name: []const u8) ![]const u8 {
        const source_cstr = try self.allocator.dupeZ(u8, contract_source);
        defer self.allocator.free(source_cstr);

        const name_cstr = try self.allocator.dupeZ(u8, name);
        defer self.allocator.free(name_cstr);

        const result = c.sol_parser_parse(self.ctx, source_cstr.ptr, name_cstr.ptr);
        if (result == null) {
            const errors = c.sol_parser_get_errors(self.ctx);
            if (errors != null) {
                defer c.sol_free_string(errors);
                std.debug.print("Parse errors:\n{s}\n", .{std.mem.span(errors)});
            }
            return error.ParseFailed;
        }

        const result_str = std.mem.span(result);
        const owned = try self.allocator.dupe(u8, result_str);
        c.sol_free_string(result);
        return owned;
    }

    /// Parse just a function definition (no contract wrapper needed!)
    /// This is the key - we parse the function in isolation
    pub fn parseFunctionDirect(self: *Self, function_source: []const u8, name: []const u8) ![]const u8 {
        // Wrap function minimally just to make it syntactically valid
        const wrapped = try std.fmt.allocPrint(
            self.allocator,
            \\// SPDX-License-Identifier: UNLICENSED
            \\pragma solidity ^0.8.0;
            \\contract __ShadowTemp__ {{
            \\    {s}
            \\}}
            \\
        ,
            .{function_source},
        );
        defer self.allocator.free(wrapped);

        return self.parseContract(wrapped, name);
    }

    /// Extract just the function node from a parsed AST
    pub fn extractFunctionNode(self: *Self, ast_json: []const u8) !std.json.Value {
        var parser = std.json.Parser.init(self.allocator, .alloc_always);
        defer parser.deinit();

        var tree = try parser.parse(ast_json);
        defer tree.deinit();

        // Navigate: root -> nodes[1] (contract) -> nodes[0..n] (members)
        if (tree.root.object.get("nodes")) |nodes_value| {
            if (nodes_value.array.items.len > 1) {
                const contract = nodes_value.array.items[1];
                if (contract.object.get("nodes")) |contract_nodes| {
                    // Return all nodes (could be functions, state vars, etc)
                    return contract_nodes;
                }
            }
        }

        return error.FunctionNotFound;
    }

    /// Stitch a shadow function into a valid contract's AST
    pub fn stitchFunction(
        self: *Self,
        original_contract_ast: []const u8,
        shadow_function_ast: []const u8,
    ) ![]const u8 {
        var original_parser = std.json.Parser.init(self.allocator, .alloc_always);
        defer original_parser.deinit();

        var shadow_parser = std.json.Parser.init(self.allocator, .alloc_always);
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
            try contract_nodes_ptr.array.append(self.allocator, shadow_function_node);
        } else {
            return error.ContractNodesNotFound;
        }

        // Serialize back to JSON
        var output = std.ArrayList(u8).init(self.allocator);
        defer output.deinit();

        try std.json.stringify(original_tree.root, .{}, output.writer());
        return output.toOwnedSlice();
    }
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    std.debug.print("=== Shadow Function Stitching Test ===\n\n", .{});

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
    // This won't compile in the original contract, but we can parse it!
    const shadow_function =
        \\function shadowExploit() public view returns (uint) {
        \\    return secretValue + 100;
        \\}
    ;

    var stitcher = try ShadowStitcher.init(allocator);
    defer stitcher.deinit();

    std.debug.print("Step 1: Parse original valid contract\n", .{});
    const original_ast = try stitcher.parseContract(original_contract, "MyContract.sol");
    defer allocator.free(original_ast);
    std.debug.print("✓ Original contract parsed ({} bytes)\n\n", .{original_ast.len});

    std.debug.print("Step 2: Parse shadow function (references private secretValue)\n", .{});
    const shadow_ast = try stitcher.parseFunctionDirect(shadow_function, "Shadow.sol");
    defer allocator.free(shadow_ast);
    std.debug.print("✓ Shadow function parsed ({} bytes)\n", .{shadow_ast.len});
    std.debug.print("   Note: Parser doesn't care that secretValue isn't defined!\n\n", .{});

    std.debug.print("Step 3: Stitch shadow function into original contract's AST\n", .{});
    const stitched_ast = try stitcher.stitchFunction(original_ast, shadow_ast);
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
        std.debug.print("  - Original contract's private variable 'secretValue'\n", .{});
        std.debug.print("  - Original contract's function 'getOwner()'\n", .{});
        std.debug.print("  - Shadow function 'shadowExploit()' that accesses secretValue\n", .{});
        std.debug.print("\n  The parser doesn't validate that secretValue is accessible!\n", .{});
    } else {
        std.debug.print("\n✗ FAILED: Stitching incomplete\n", .{});
        return error.StitchingFailed;
    }

    // Save the stitched AST to a file for inspection
    std.debug.print("\nSaving stitched AST to 'stitched_ast.json'\n", .{});
    const file = try std.fs.cwd().createFile("stitched_ast.json", .{});
    defer file.close();
    try file.writeAll(stitched_ast);
    std.debug.print("✓ Saved!\n", .{});
}
