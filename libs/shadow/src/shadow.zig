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
//! // Option 1: Stitch into source code (auto-selects last contract)
//! const target_source =
//!     \\// SPDX-License-Identifier: MIT
//!     \\pragma solidity ^0.8.0;
//!     \\contract MyContract {
//!     \\    uint private secretValue;
//!     \\    function getSecret() public view returns (uint) { return secretValue; }
//!     \\}
//! ;
//! const analyzed_ast = try shadow.stitchIntoSource(target_source, null, null);
//! defer allocator.free(analyzed_ast);
//!
//! // Option 2: Stitch into specific contract by name
//! const analyzed_ast2 = try shadow.stitchIntoSource(target_source, null, "MyContract");
//! defer allocator.free(analyzed_ast2);
//!
//! // Option 3: Stitch into existing AST
//! const target_ast = try Shadow.parseSourceAst(allocator, target_source, null);
//! defer allocator.free(target_ast);
//! const analyzed_ast3 = try shadow.stitchIntoAst(target_ast, null); // null = auto-select last contract
//! defer allocator.free(analyzed_ast3);
//! ```

const std = @import("std");
const Utils = @import("utils.zig").Utils;

const c = @cImport({
    @cInclude("solidity-parser-wrapper.h");
});

pub const Shadow = struct {
    allocator: std.mem.Allocator,
    source: []const u8, // owned copy of the source
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

        // IMPORTANT: Make a copy of the source string so we own the memory
        // The source pointer from JavaScript is temporary and gets reused
        const source_copy = allocator.dupe(u8, source) catch |err| {
            c.sol_parser_destroy(ctx);
            return err;
        };

        return Self{
            .allocator = allocator,
            .source = source_copy,
            .ctx = ctx,
        };
    }

    pub fn deinit(self: *Self) void {
        // Free the owned source copy
        self.allocator.free(self.source);
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
        const root_object = switch (parsed.value) {
            .object => |obj| obj,
            else => return Error.InvalidContractStructure,
        };

        if (root_object.get("nodes")) |nodes_value| {
            const nodes_array = switch (nodes_value) {
                .array => |arr| arr,
                else => return Error.InvalidContractStructure,
            };

            if (nodes_array.items.len > 1) {
                const contract = nodes_array.items[1];
                const contract_object = switch (contract) {
                    .object => |obj| obj,
                    else => return Error.InvalidContractStructure,
                };

                if (contract_object.get("nodes")) |contract_nodes| {
                    const contract_nodes_array = switch (contract_nodes) {
                        .array => |arr| arr,
                        else => return Error.InvalidContractStructure,
                    };

                    const node_count = contract_nodes_array.items.len;
                    if (node_count == 0) return Error.NoNodesFound;

                    var nodes = try self.allocator.alloc([]const u8, node_count);
                    for (contract_nodes_array.items, 0..) |node, i| {
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

        const source_cstr = allocator.allocSentinel(u8, source.len, 0) catch |err| {
            c.sol_parser_destroy(ctx);
            return err;
        };
        defer allocator.free(source_cstr);
        @memcpy(source_cstr[0..source.len], source);

        const source_name_raw = name orelse "Contract.sol";
        const source_name_cstr = try allocator.dupeZ(u8, source_name_raw);
        defer allocator.free(source_name_cstr);

        const result = c.sol_parser_parse(ctx, source_cstr.ptr, source_name_cstr.ptr);

        if (result == null) {
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
    ///
    /// Parameters:
    ///   - target_ast: The parsed AST JSON of the target source
    ///   - target_contract_name: Optional contract name. If null, stitches into the last ContractDefinition.
    ///                           If specified, finds and stitches into the named contract.
    pub fn stitchIntoAst(self: *Self, target_ast: []const u8, target_contract_name: ?[]const u8) Error![]const u8 {
        // Parse shadow and target ASTs
        const shadow_ast = try self.toWrappedAst();
        defer self.allocator.free(shadow_ast);

        var target_parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, target_ast, .{});
        defer target_parsed.deinit();

        var shadow_parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, shadow_ast, .{});
        defer shadow_parsed.deinit();

        // Find contract and stitch
        const target_object = switch (target_parsed.value) {
            .object => |obj| obj,
            else => return Error.InvalidContractStructure,
        };
        const nodes = target_object.get("nodes") orelse return Error.InvalidContractStructure;
        const max_target_id = Utils.findMaxId(target_parsed.value);
        const contract_idx = try Self.findTargetContractIndex(nodes, target_contract_name);

        const nodes_array = switch (nodes) {
            .array => |arr| arr,
            else => return Error.InvalidContractStructure,
        };
        const target_contract = &nodes_array.items[contract_idx];
        try Self.stitchShadowNodesIntoContract(target_contract, shadow_parsed.value, max_target_id);

        // Analyze and return
        const source_name = target_contract_name orelse "Contract.sol";
        return try self.analyzeAst(target_parsed.value, source_name);
    }

    /// Stitch shadow function(s) into an existing contract's source code
    /// Convenience wrapper that parses the source first, then stitches ASTs
    /// Returns fully analyzed AST with type information, scope resolution, and reference linkage
    ///
    /// Parameters:
    ///   - target_source: The Solidity source code
    ///   - source_name: Optional source file name (for error messages)
    ///   - target_contract_name: Optional contract name. If null, stitches into the last ContractDefinition.
    pub fn stitchIntoSource(
        self: *Self,
        target_source: []const u8,
        source_name: ?[]const u8,
        target_contract_name: ?[]const u8,
    ) Error![]const u8 {
        const target_ast = try parseSourceAst(self.allocator, target_source, source_name);
        defer self.allocator.free(target_ast);
        return try self.stitchIntoAst(target_ast, target_contract_name);
    }

    // ========================================================================
    // Internal Helper Functions
    // ========================================================================

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

    /// Find the index of the target contract in the AST nodes array
    /// Returns the index, or error if not found
    fn findTargetContractIndex(nodes: std.json.Value, contract_name: ?[]const u8) Error!usize {
        const nodes_array = switch (nodes) {
            .array => |arr| arr,
            else => return Error.InvalidContractStructure,
        };

        if (contract_name) |name| {
            // Explicit: Find contract by name
            for (nodes_array.items, 0..) |node, i| {
                if (!isContractDefinition(node)) continue;
                if (getContractName(node)) |node_name| {
                    if (std.mem.eql(u8, node_name, name)) return i;
                }
            }
            return Error.InvalidContractStructure;
        } else {
            // Heuristic: Use last ContractDefinition
            // Works because derived contracts, implementations, and contract users typically come last
            var last_idx: ?usize = null;
            for (nodes_array.items, 0..) |node, i| {
                if (isContractDefinition(node)) last_idx = i;
            }
            return last_idx orelse return Error.InvalidContractStructure;
        }
    }

    /// Stitch shadow contract nodes into target contract
    /// Renumbers shadow IDs and appends shadow nodes to target
    fn stitchShadowNodesIntoContract(
        target_contract: *std.json.Value,
        shadow_parsed: std.json.Value,
        max_target_id: i64,
    ) Error!void {
        const shadow_object = switch (shadow_parsed) {
            .object => |obj| obj,
            else => return Error.InvalidContractStructure,
        };
        const shadow_nodes = shadow_object.get("nodes") orelse return Error.InvalidContractStructure;

        const shadow_nodes_array = switch (shadow_nodes) {
            .array => |arr| arr,
            else => return Error.InvalidContractStructure,
        };

        if (shadow_nodes_array.items.len <= 1) return Error.NoNodesFound;

        const shadow_contract = &shadow_nodes_array.items[1];

        // Renumber IDs to avoid collisions
        Utils.renumberIds(shadow_contract, max_target_id);

        // Get the nodes arrays
        const target_contract_object = switch (target_contract.*) {
            .object => |*obj| obj,
            else => return Error.InvalidContractStructure,
        };
        const target_nodes = target_contract_object.getPtr("nodes") orelse return Error.InvalidContractStructure;

        const target_nodes_array = switch (target_nodes.*) {
            .array => |*arr| arr,
            else => return Error.InvalidContractStructure,
        };

        const shadow_contract_object = switch (shadow_contract.*) {
            .object => |obj| obj,
            else => return Error.NoNodesFound,
        };
        const shadow_contract_nodes = shadow_contract_object.get("nodes") orelse return Error.NoNodesFound;

        const shadow_contract_nodes_array = switch (shadow_contract_nodes) {
            .array => |arr| arr,
            else => return Error.NoNodesFound,
        };

        // Append each shadow node to target
        for (shadow_contract_nodes_array.items) |shadow_node| {
            try target_nodes_array.append(shadow_node);
        }
    }

    /// Serialize AST to JSON and run semantic analysis
    /// Returns the analyzed AST as JSON string
    fn analyzeAst(
        self: *Self,
        ast: std.json.Value,
        source_name: []const u8,
    ) Error![]const u8 {
        // Serialize to JSON
        const json_str = try std.fmt.allocPrint(
            self.allocator,
            "{f}",
            .{std.json.fmt(ast, .{})},
        );
        defer self.allocator.free(json_str);

        // Convert to C strings
        const json_cstr = try self.allocator.dupeZ(u8, json_str);
        defer self.allocator.free(json_cstr);

        const source_name_cstr = try self.allocator.dupeZ(u8, source_name);
        defer self.allocator.free(source_name_cstr);

        // Analyze via C++
        const result = c.sol_analyze_parsed_ast_json(
            self.ctx,
            json_cstr.ptr,
            source_name_cstr.ptr,
        );

        if (result == null) {
            return Error.AnalysisFailed;
        }

        const result_str = std.mem.span(result);
        const owned = try self.allocator.dupe(u8, result_str);
        c.sol_free_string(result);
        return owned;
    }

    /// Check if a JSON node is a ContractDefinition
    fn isContractDefinition(node: std.json.Value) bool {
        const node_object = switch (node) {
            .object => |obj| obj,
            else => return false,
        };
        if (node_object.get("nodeType")) |node_type| {
            switch (node_type) {
                .string => |s| return std.mem.eql(u8, s, "ContractDefinition"),
                else => return false,
            }
        }
        return false;
    }

    /// Get the name of a contract node
    fn getContractName(node: std.json.Value) ?[]const u8 {
        const node_object = switch (node) {
            .object => |obj| obj,
            else => return null,
        };
        if (node_object.get("name")) |name| {
            switch (name) {
                .string => |s| return s,
                else => return null,
            }
        }
        return null;
    }
};
