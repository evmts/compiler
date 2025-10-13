const std = @import("std");
const Shadow = @import("shadow.zig").Shadow;

test "parse function with undefined variable" {
    const allocator = std.testing.allocator;

    const bad_function =
        \\function badFunction() public returns (uint) {
        \\    return undefinedVariable + 5;
        \\}
    ;

    var shadow = try Shadow.init(allocator, bad_function);
    defer shadow.deinit();

    const ast = try shadow.parseToAST();
    defer allocator.free(ast);

    // Should successfully parse and return non-empty AST
    try std.testing.expect(ast.len > 0);

    // AST should contain the function name
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

    const ast = try shadow.parseToAST();
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

    const ast = try shadow.parseToAST();
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

    const ast = try shadow.parseToAST();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "complexFunction") != null);
    // All undefined variables should appear in AST as identifiers
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

    const ast = try shadow.parseToAST();
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

    const ast = try shadow.parseToAST();
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "validFunction") != null);
}

test "parse function with syntax error should fail" {
    const allocator = std.testing.allocator;

    // Missing closing brace - syntax error, not semantic
    const syntax_error =
        \\function syntaxError() public {
        \\    return 42;
    ;

    var shadow = try Shadow.init(allocator, syntax_error);
    defer shadow.deinit();

    // Should fail because syntax is invalid
    const result = shadow.parseToAST();
    try std.testing.expectError(error.ParseFailed, result);
}

test "Shadow init and deinit" {
    const allocator = std.testing.allocator;

    const simple_func = "function test() public {}";

    var shadow = try Shadow.init(allocator, simple_func);
    shadow.deinit();

    // Just verifying no memory leaks
    try std.testing.expect(true);
}

test "stitch shadow function into valid contract" {
    const allocator = std.testing.allocator;

    // Valid contract with private variables
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

    // Shadow function that accesses the private variable
    const shadow_function =
        \\function exploit() public view returns (uint) {
        \\    return secretValue * 2;
        \\}
    ;

    // Parse original contract
    const original_ast = try Shadow.parseFullContract(allocator, original_contract);
    defer allocator.free(original_ast);

    // Parse shadow function
    var shadow = try Shadow.init(allocator, shadow_function);
    defer shadow.deinit();
    const shadow_ast = try shadow.parseToAST();
    defer allocator.free(shadow_ast);

    // Stitch them together
    const stitched_ast = try Shadow.stitchIntoContract(allocator, original_ast, shadow_ast);
    defer allocator.free(stitched_ast);

    // Verify both functions are in the stitched AST
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

    const ast = try Shadow.parseFullContract(allocator, contract);
    defer allocator.free(ast);

    try std.testing.expect(ast.len > 0);
    try std.testing.expect(std.mem.indexOf(u8, ast, "Test") != null);
    try std.testing.expect(std.mem.indexOf(u8, ast, "foo") != null);
}
