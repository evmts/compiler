//! Shadow - Parse and stitch Solidity code fragments into contract ASTs
//!
//! Shadow enables parsing incomplete Solidity code (functions, variables, etc.)
//! and stitching them into existing contracts without requiring semantic validity
//! upfront. Demonstrates that Solidity's parser performs pure syntax analysis,
//! allowing AST manipulation before semantic validation.
//!
//! Usage Example:
//! ```zig
//! const allocator = std.heap.page_allocator;
//!
//! // Create shadow from function fragment
//! const shadow_fn = "function exploit() public view returns (uint) { return secretValue * 2; }";
//! var shadow = try Shadow.init(allocator, shadow_fn);
//! defer shadow.deinit();
//!
//! // Option 1: Stitch into source code
//! const target_source =
//!     \\// SPDX-License-Identifier: MIT
//!     \\pragma solidity ^0.8.0;
//!     \\contract MyContract {
//!     \\    uint private secretValue;
//!     \\    function getSecret() public view returns (uint) { return secretValue; }
//!     \\}
//! ;
//! const analyzed_ast = try shadow.stitchIntoSource(target_source, null);
//! defer allocator.free(analyzed_ast);
//!
//! // Option 2: Stitch into existing AST
//! const target_ast = try Shadow.parseSourceAst(allocator, target_source, null);
//! defer allocator.free(target_ast);
//! const analyzed_ast2 = try shadow.stitchIntoAst(target_ast);
//! defer allocator.free(analyzed_ast2);
//! ```

const std = @import("std");
const Utils = @import("utils.zig").Utils;

const c = @cImport({
    @cInclude("solidity-parser-wrapper.h");
});

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
        /// Semantic analysis failed
        AnalysisFailed,
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

    /// Reconstruct source AST from individual AST nodes
    /// TODO: Implement this function to reverse toAstNodes() operation
    /// Takes AST node JSON strings and reconstructs a complete source AST
    /// Should store and use the original source AST structure
    pub fn fromAstNodes(self: *Self, nodes: []const []const u8) Error![]const u8 {
        _ = self;
        _ = nodes;
        @panic("TODO: Implement fromAstNodes");
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
    /// Assembles ASTs in Zig (JSON manipulation), then analyzes the stitched result
    /// Returns fully analyzed AST with type information, scope resolution, and reference linkage
    pub fn stitchIntoAst(self: *Self, target_ast: []const u8) Error![]const u8 {
        // Get parsed shadow AST
        const shadow_ast = try self.toWrappedAst();
        defer self.allocator.free(shadow_ast);

        // Parse both JSONs
        var target_parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, target_ast, .{});
        defer target_parsed.deinit();

        var shadow_parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, shadow_ast, .{});
        defer shadow_parsed.deinit();

        // Find max ID in target to avoid collisions
        const max_target_id = Utils.findMaxId(target_parsed.value);

        // Navigate to contract nodes and stitch
        // Structure: root -> nodes[1] (contract) -> nodes[...] (functions)
        if (target_parsed.value.object.get("nodes")) |target_nodes_value| {
            if (target_nodes_value.array.items.len > 1) {
                const target_contract = &target_nodes_value.array.items[1];

                if (shadow_parsed.value.object.get("nodes")) |shadow_nodes_value| {
                    if (shadow_nodes_value.array.items.len > 1) {
                        var shadow_contract = &shadow_nodes_value.array.items[1];
                        // Renumber shadow IDs to avoid collisions (offset by max target ID)
                        Utils.renumberIds(shadow_contract, max_target_id);

                        // Get contract nodes arrays
                        if (target_contract.object.getPtr("nodes")) |target_contract_nodes| {
                            if (shadow_contract.object.get("nodes")) |shadow_contract_nodes| {
                                // Append shadow functions to target
                                for (shadow_contract_nodes.array.items) |shadow_node| {
                                    try target_contract_nodes.array.append(shadow_node);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Serialize stitched AST back to JSON string
        const stitched_json = try std.fmt.allocPrint(self.allocator, "{f}", .{std.json.fmt(target_parsed.value, .{})});
        defer self.allocator.free(stitched_json);

        // Convert to null-terminated string for C
        const stitched_cstr = try self.allocator.dupeZ(u8, stitched_json);
        defer self.allocator.free(stitched_cstr);

        // Call C++ to analyze the stitched AST
        const result = c.sol_analyze_parsed_ast_json(
            self.ctx,
            stitched_cstr.ptr,
            "Contract.sol",
        );

        if (result == null) {
            const errors = c.sol_parser_get_errors(self.ctx);
            if (errors != null) {
                defer c.sol_free_string(errors);
                const err_str = std.mem.span(errors);
                std.debug.print("Analysis errors:\n{s}\n", .{err_str});
            } else {
                std.debug.print("Analysis failed but no error message available\n", .{});
            }
            return Error.AnalysisFailed;
        }

        // Copy result and return
        const result_str = std.mem.span(result);
        const owned = try self.allocator.dupe(u8, result_str);
        c.sol_free_string(result);
        return owned;
    }

    /// Stitch shadow function(s) into an existing contract's source code
    /// Convenience wrapper that parses the source first, then stitches ASTs
    /// Returns fully analyzed AST with type information, scope resolution, and reference linkage
    pub fn stitchIntoSource(self: *Self, target_source: []const u8, name: ?[]const u8) Error![]const u8 {
        const target_ast = try parseSourceAst(self.allocator, target_source, name);
        defer self.allocator.free(target_ast);
        return try self.stitchIntoAst(target_ast);
    }

    /// Internal: Parse shadow source wrapped in minimal contract boilerplate
    /// Returns JSON representation of the AST
    fn toWrappedAst(self: *Self) Error![]const u8 {
        // Create minimal boilerplate that makes it syntactically valid
        // Note: This code won't COMPILE due to semantic errors, but it will PARSE!
        const wrapped = try std.fmt.allocPrint(
            self.allocator,
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
        defer self.allocator.free(wrapped);
        return try parseSourceAst(self.allocator, wrapped, "Shadow.sol");
    }
};
