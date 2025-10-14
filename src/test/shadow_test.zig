const std = @import("std");
const Shadow = @import("shadow").Shadow;
const testing = std.testing;

// ============================================================================
// Initialization & Lifecycle Tests
// ============================================================================

test "Shadow.init - basic initialization and cleanup" {
    const allocator = testing.allocator;
    const simple_func = "function test() public {}";

    var shadow = try Shadow.init(allocator, simple_func);
    defer shadow.deinit();

    try testing.expect(shadow.source.len > 0);
}

test "Shadow.init - with complex function" {
    const allocator = testing.allocator;
    const complex_func =
        \\function compute(uint x, uint y) public pure returns (uint) {
        \\    return x * y + 100;
        \\}
    ;

    var shadow = try Shadow.init(allocator, complex_func);
    defer shadow.deinit();

    try testing.expect(shadow.source.len > 0);
}

test "Shadow.init - with multiple functions" {
    const allocator = testing.allocator;
    const multi_funcs =
        \\function alpha() public {}
        \\function beta() private view returns (uint) { return 42; }
        \\function gamma() external payable {}
    ;

    var shadow = try Shadow.init(allocator, multi_funcs);
    defer shadow.deinit();

    try testing.expect(shadow.source.len > 0);
}

// ============================================================================
// parseSourceAst Tests
// ============================================================================

test "parseSourceAst - valid simple contract" {
    const allocator = testing.allocator;
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

    try testing.expect(ast.len > 0);
    try testing.expect(std.mem.indexOf(u8, ast, "Test") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "foo") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "\"nodeType\"") != null);
}

test "parseSourceAst - contract with state variables" {
    const allocator = testing.allocator;
    const contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract Storage {
        \\    uint private data;
        \\    address public owner;
        \\
        \\    function getData() public view returns (uint) {
        \\        return data;
        \\    }
        \\}
    ;

    const ast = try Shadow.parseSourceAst(allocator, contract, null);
    defer allocator.free(ast);

    try testing.expect(std.mem.indexOf(u8, ast, "Storage") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "data") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "owner") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "getData") != null);
}

test "parseSourceAst - contract with various function types" {
    const allocator = testing.allocator;
    const contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract Functions {
        \\    uint value;
        \\
        \\    function pureFunc(uint x) public pure returns (uint) { return x * 2; }
        \\    function viewFunc() public view returns (uint) { return value; }
        \\    function payableFunc() public payable {}
        \\    function externalFunc() external {}
        \\    function internalFunc() internal {}
        \\    function privateFunc() private {}
        \\}
    ;

    const ast = try Shadow.parseSourceAst(allocator, contract, null);
    defer allocator.free(ast);

    try testing.expect(std.mem.indexOf(u8, ast, "pureFunc") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "viewFunc") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "payableFunc") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "externalFunc") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "internalFunc") != null);
    try testing.expect(std.mem.indexOf(u8, ast, "privateFunc") != null);
}

test "parseSourceAst - invalid syntax should fail" {
    const allocator = testing.allocator;
    const invalid_contract =
        \\pragma solidity ^0.8.0;
        \\contract Broken {
        \\    function broken( public {}
        \\}
    ;

    const result = Shadow.parseSourceAst(allocator, invalid_contract, null);
    try testing.expectError(Shadow.Error.ParseFailed, result);
}

test "parseSourceAst - verify JSON structure" {
    const allocator = testing.allocator;
    const contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract Test {}
    ;

    const ast = try Shadow.parseSourceAst(allocator, contract, null);
    defer allocator.free(ast);

    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, ast, .{});
    defer parsed.deinit();

    try testing.expect(parsed.value.object.get("nodeType") != null);
    try testing.expect(parsed.value.object.get("nodes") != null);
}

// ============================================================================
// toAstNodes Tests
// ============================================================================

test "toAstNodes - single function extraction" {
    const allocator = testing.allocator;
    const single_func = "function test() public {}";

    var shadow = try Shadow.init(allocator, single_func);
    defer shadow.deinit();

    const nodes = try shadow.toAstNodes();
    defer {
        for (nodes) |node| allocator.free(node);
        allocator.free(nodes);
    }

    try testing.expectEqual(@as(usize, 1), nodes.len);
    try testing.expect(std.mem.indexOf(u8, nodes[0], "test") != null);
    try testing.expect(std.mem.indexOf(u8, nodes[0], "\"nodeType\"") != null);
}

test "toAstNodes - multiple functions extraction" {
    const allocator = testing.allocator;
    const multi_funcs =
        \\function alpha() public {}
        \\function beta() public {}
        \\function gamma() public {}
    ;

    var shadow = try Shadow.init(allocator, multi_funcs);
    defer shadow.deinit();

    const nodes = try shadow.toAstNodes();
    defer {
        for (nodes) |node| allocator.free(node);
        allocator.free(nodes);
    }

    try testing.expectEqual(@as(usize, 3), nodes.len);

    var found_alpha = false;
    var found_beta = false;
    var found_gamma = false;

    for (nodes) |node| {
        if (std.mem.indexOf(u8, node, "alpha") != null) found_alpha = true;
        if (std.mem.indexOf(u8, node, "beta") != null) found_beta = true;
        if (std.mem.indexOf(u8, node, "gamma") != null) found_gamma = true;
    }

    try testing.expect(found_alpha);
    try testing.expect(found_beta);
    try testing.expect(found_gamma);
}

test "toAstNodes - functions with different visibilities" {
    const allocator = testing.allocator;
    const funcs =
        \\function publicFunc() public {}
        \\function externalFunc() external {}
        \\function internalFunc() internal {}
        \\function privateFunc() private {}
    ;

    var shadow = try Shadow.init(allocator, funcs);
    defer shadow.deinit();

    const nodes = try shadow.toAstNodes();
    defer {
        for (nodes) |node| allocator.free(node);
        allocator.free(nodes);
    }

    try testing.expectEqual(@as(usize, 4), nodes.len);
}

test "toAstNodes - verify JSON validity of extracted nodes" {
    const allocator = testing.allocator;
    const func = "function test(uint x) public pure returns (uint) { return x; }";

    var shadow = try Shadow.init(allocator, func);
    defer shadow.deinit();

    const nodes = try shadow.toAstNodes();
    defer {
        for (nodes) |node| allocator.free(node);
        allocator.free(nodes);
    }

    for (nodes) |node| {
        var parsed = try std.json.parseFromSlice(std.json.Value, allocator, node, .{});
        defer parsed.deinit();

        try testing.expect(parsed.value.object.get("nodeType") != null);
        try testing.expect(parsed.value.object.get("id") != null);
    }
}

// ============================================================================
// stitchIntoAst Tests
// ============================================================================

test "stitchIntoAst - basic stitching" {
    const allocator = testing.allocator;

    const target_contract =
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

    const shadow_func = "function exploit() public view returns (uint) { return secretValue * 2; }";

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "getSecret") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "exploit") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "secretValue") != null);
}

test "stitchIntoAst - multiple shadow functions" {
    const allocator = testing.allocator;

    const target_contract =
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

    const shadow_funcs =
        \\function exploitOne() public view returns (uint) {
        \\    return data * 2;
        \\}
        \\
        \\function exploitTwo() public view returns (uint) {
        \\    return data + 100;
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_funcs);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "getData") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "exploitOne") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "exploitTwo") != null);
}

test "stitchIntoAst - verify semantic analysis succeeded" {
    const allocator = testing.allocator;

    const target_contract =
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

    const shadow_func = "function exploit() public view returns (uint) { return secretValue * 2; }";

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    // Semantic analysis adds these annotations
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "\"typeDescriptions\"") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "\"referencedDeclaration\"") != null);
}

test "stitchIntoAst - verify no duplicate IDs" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract MyContract {
        \\    uint value;
        \\    function getA() public view returns (uint) { return value; }
        \\    function getB() public view returns (uint) { return value; }
        \\}
    ;

    const shadow_func = "function getC() public view returns (uint) { return value; }";

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    // Parse and verify no duplicate IDs
    var parsed = try std.json.parseFromSlice(std.json.Value, allocator, analyzed_ast, .{});
    defer parsed.deinit();

    var id_set = std.AutoHashMap(i64, void).init(allocator);
    defer id_set.deinit();

    try verifyUniqueIds(parsed.value, &id_set);
}

test "stitchIntoAst - shadow accessing private variables" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract Vault {
        \\    uint private balance;
        \\    mapping(address => uint) private deposits;
        \\
        \\    function deposit() public payable {
        \\        deposits[msg.sender] += msg.value;
        \\        balance += msg.value;
        \\    }
        \\}
    ;

    const shadow_func =
        \\function stealBalance() public view returns (uint) {
        \\    return balance;
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "stealBalance") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "balance") != null);
}

test "stitchIntoAst - target with modifier" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract Guarded {
        \\    address private owner;
        \\    uint private value;
        \\
        \\    modifier onlyOwner() {
        \\        require(msg.sender == owner);
        \\        _;
        \\    }
        \\
        \\    function setValue(uint v) public onlyOwner {
        \\        value = v;
        \\    }
        \\}
    ;

    const shadow_func =
        \\function bypassGuard() public view returns (address) {
        \\    return owner;
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "bypassGuard") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "onlyOwner") != null);
}

// ============================================================================
// stitchIntoSource Tests
// ============================================================================

test "stitchIntoSource - basic usage" {
    const allocator = testing.allocator;

    const target_source =
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

    const shadow_func = "function exploit() public view returns (uint) { return secretValue * 2; }";

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoSource(target_source, null);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "getSecret") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "exploit") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "\"typeDescriptions\"") != null);
}

test "stitchIntoSource - verify equivalence to stitchIntoAst" {
    const allocator = testing.allocator;

    const target_source =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract Test {
        \\    uint value;
        \\    function get() public view returns (uint) { return value; }
        \\}
    ;

    const shadow_func = "function set(uint v) public { value = v; }";

    var shadow1 = try Shadow.init(allocator, shadow_func);
    defer shadow1.deinit();

    const result1 = try shadow1.stitchIntoSource(target_source, null);
    defer allocator.free(result1);

    const target_ast = try Shadow.parseSourceAst(allocator, target_source, null);
    defer allocator.free(target_ast);

    var shadow2 = try Shadow.init(allocator, shadow_func);
    defer shadow2.deinit();

    const result2 = try shadow2.stitchIntoAst(target_ast);
    defer allocator.free(result2);

    // Both should contain same function names
    try testing.expect(std.mem.indexOf(u8, result1, "get") != null);
    try testing.expect(std.mem.indexOf(u8, result1, "set") != null);
    try testing.expect(std.mem.indexOf(u8, result2, "get") != null);
    try testing.expect(std.mem.indexOf(u8, result2, "set") != null);
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

test "edge case - shadow with complex expressions" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract Math {
        \\    uint a;
        \\    uint b;
        \\}
    ;

    const shadow_func =
        \\function complexMath() public view returns (uint) {
        \\    return (a * b + 100) / (a + b - 1) % 7;
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "complexMath") != null);
}

test "edge case - shadow with conditional logic" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract Logic {
        \\    bool flag;
        \\    uint value;
        \\}
    ;

    const shadow_func =
        \\function conditional() public view returns (uint) {
        \\    if (flag) {
        \\        return value * 2;
        \\    } else {
        \\        return value / 2;
        \\    }
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "conditional") != null);
}

test "edge case - target with many existing functions" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract Many {
        \\    uint value;
        \\    function f1() public view returns (uint) { return value; }
        \\    function f2() public view returns (uint) { return value; }
        \\    function f3() public view returns (uint) { return value; }
        \\    function f4() public view returns (uint) { return value; }
        \\    function f5() public view returns (uint) { return value; }
        \\}
    ;

    const shadow_func = "function f6() public view returns (uint) { return value * 10; }";

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "f1") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "f6") != null);
}

// ============================================================================
// Complex Features Tests
// ============================================================================

test "complex - inheritance" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\
        \\contract Base {
        \\    uint internal baseValue;
        \\}
        \\
        \\contract Derived is Base {
        \\    uint private derivedValue;
        \\
        \\    function getValue() public view returns (uint) {
        \\        return derivedValue;
        \\    }
        \\}
    ;

    const shadow_func =
        \\function accessBoth() public view returns (uint) {
        \\    return baseValue + derivedValue;
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "accessBoth") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "baseValue") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "derivedValue") != null);
}

test "complex - struct definitions" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract WithStruct {
        \\    struct User {
        \\        address addr;
        \\        uint balance;
        \\        bool active;
        \\    }
        \\    mapping(address => User) users;
        \\}
    ;

    const shadow_func =
        \\function getUser(address a) public view returns (User memory) {
        \\    return users[a];
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "getUser") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "User") != null);
}

test "complex - events" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract WithEvents {
        \\    event Transfer(address indexed from, address indexed to, uint value);
        \\    uint balance;
        \\}
    ;

    const shadow_func =
        \\function emitTransfer(address to, uint amount) public {
        \\    balance -= amount;
        \\    emit Transfer(msg.sender, to, amount);
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "emitTransfer") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "Transfer") != null);
}

test "complex - custom errors" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\contract WithErrors {
        \\    error InsufficientBalance(uint available, uint required);
        \\    uint balance;
        \\}
    ;

    const shadow_func =
        \\function withdraw(uint amount) public {
        \\    if (balance < amount) {
        \\        revert InsufficientBalance(balance, amount);
        \\    }
        \\    balance -= amount;
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "withdraw") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "InsufficientBalance") != null);
}

test "complex - interfaces" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\interface IERC20 {
        \\    function transfer(address to, uint amount) external returns (bool);
        \\}
        \\contract TokenUser {
        \\    IERC20 token;
        \\}
    ;

    const shadow_func =
        \\function sendTokens(address to, uint amount) public {
        \\    token.transfer(to, amount);
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "sendTokens") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "IERC20") != null);
}

test "complex - library usage" {
    const allocator = testing.allocator;

    const target_contract =
        \\// SPDX-License-Identifier: MIT
        \\pragma solidity ^0.8.0;
        \\library SafeMath {
        \\    function add(uint a, uint b) internal pure returns (uint) {
        \\        return a + b;
        \\    }
        \\}
        \\contract UseLibrary {
        \\    using SafeMath for uint;
        \\    uint value;
        \\}
    ;

    const shadow_func =
        \\function increment(uint amount) public {
        \\    value = value.add(amount);
        \\}
    ;

    const target_ast = try Shadow.parseSourceAst(allocator, target_contract, null);
    defer allocator.free(target_ast);

    var shadow = try Shadow.init(allocator, shadow_func);
    defer shadow.deinit();

    const analyzed_ast = try shadow.stitchIntoAst(target_ast);
    defer allocator.free(analyzed_ast);

    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "increment") != null);
    try testing.expect(std.mem.indexOf(u8, analyzed_ast, "SafeMath") != null);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn verifyUniqueIds(value: std.json.Value, id_set: *std.AutoHashMap(i64, void)) !void {
    switch (value) {
        .object => |obj| {
            if (obj.get("id")) |id_value| {
                if (id_value == .integer) {
                    const id = id_value.integer;
                    if (id_set.contains(id)) {
                        std.debug.print("Duplicate ID found: {d}\n", .{id});
                        return error.DuplicateId;
                    }
                    try id_set.put(id, {});
                }
            }
            var it = obj.iterator();
            while (it.next()) |entry| {
                try verifyUniqueIds(entry.value_ptr.*, id_set);
            }
        },
        .array => |arr| {
            for (arr.items) |item| {
                try verifyUniqueIds(item, id_set);
            }
        },
        else => {},
    }
}
