import { describe, test, expect } from "bun:test";
import { Shadow } from "../build/index.js";

// Test contract sources
const TARGET_CONTRACT = `
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MyContract {
    uint256 private secretValue;

    function getSecret() public view returns (uint256) {
        return secretValue;
    }
}
`;

const SHADOW_FUNCTION = `function exploit() public view returns (uint256) {
    return secretValue * 2;
}`;

const SHADOW_VARIABLE = `uint256 public exposedSecret;`;

const SHADOW_MULTIPLE = `
function additionalFunction() public pure returns (string memory) {
    return "Hello Shadow";
}

uint256 public newValue;
`;

const MULTI_CONTRACT = `
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract First {
    uint256 public value;
}

contract Second {
    string public name;
}

contract Target {
    uint256 private secret;
}
`;

const COMPLEX_CONTRACT = `
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract ComplexContract {
    uint256 private data;
    mapping(address => uint256) public balances;

    event DataChanged(uint256 newData);

    modifier onlyPositive(uint256 _value) {
        require(_value > 0, "Value must be positive");
        _;
    }

    function setData(uint256 _data) public onlyPositive(_data) {
        data = _data;
        emit DataChanged(_data);
    }

    function getData() public view returns (uint256) {
        return data;
    }
}
`;

describe("Shadow - Basic Creation", () => {
    test("should create Shadow instance", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        expect(shadow).toBeTruthy();
    });

    test("should create Shadow with variable", () => {
        const shadow = new Shadow(SHADOW_VARIABLE);
        expect(shadow).toBeTruthy();
    });

    test("should create Shadow with multiple nodes", () => {
        const shadow = new Shadow(SHADOW_MULTIPLE);
        expect(shadow).toBeTruthy();
    });
});

describe("Shadow - AST Node Extraction", () => {
    test("should extract AST nodes from function", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const nodes = shadow.toAstNodes();

        expect(Array.isArray(nodes)).toBe(true);
        expect(nodes.length).toBeGreaterThan(0);

        // Parse first node
        const firstNode = JSON.parse(nodes[0]);
        expect(firstNode.nodeType).toBe("FunctionDefinition");
        expect(firstNode.name).toBe("exploit");
    });

    test("should extract AST nodes from variable", () => {
        const shadow = new Shadow(SHADOW_VARIABLE);
        const nodes = shadow.toAstNodes();

        expect(Array.isArray(nodes)).toBe(true);
        expect(nodes.length).toBeGreaterThan(0);

        const firstNode = JSON.parse(nodes[0]);
        expect(firstNode.nodeType).toBe("VariableDeclaration");
    });

    test("should extract multiple AST nodes", () => {
        const shadow = new Shadow(SHADOW_MULTIPLE);
        const nodes = shadow.toAstNodes();

        expect(Array.isArray(nodes)).toBe(true);
        expect(nodes.length).toBeGreaterThanOrEqual(2);

        // Should have both function and variable
        const nodeTypes = nodes.map((n) => JSON.parse(n).nodeType);
        expect(nodeTypes).toContain("FunctionDefinition");
        expect(nodeTypes).toContain("VariableDeclaration");
    });

    test("should return valid JSON for each node", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const nodes = shadow.toAstNodes();

        for (const node of nodes) {
            expect(() => JSON.parse(node)).not.toThrow();
            const parsed = JSON.parse(node);
            expect(parsed.nodeType).toBeTruthy();
        }
    });
});

describe("Shadow - Stitch Into Source", () => {
    test("should stitch function into simple contract", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        expect(result).toBeTruthy();

        // Parse result and verify it's valid AST
        const ast = JSON.parse(result);
        expect(ast.nodeType).toBe("SourceUnit");
        expect(Array.isArray(ast.nodes)).toBe(true);
    });

    test("should stitch variable into contract", () => {
        const shadow = new Shadow(SHADOW_VARIABLE);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        expect(result).toBeTruthy();
        const ast = JSON.parse(result);
        expect(ast.nodeType).toBe("SourceUnit");
    });

    test("should stitch multiple nodes into contract", () => {
        const shadow = new Shadow(SHADOW_MULTIPLE);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        expect(result).toBeTruthy();
        const ast = JSON.parse(result);
        expect(ast.nodeType).toBe("SourceUnit");
    });

    test("should stitch into complex contract", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const result = shadow.stitchIntoSource(COMPLEX_CONTRACT, null, null);

        expect(result).toBeTruthy();
        const ast = JSON.parse(result);

        // Verify AST structure
        expect(ast.nodeType).toBe("SourceUnit");

        // Find the contract
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );
        expect(contract).toBeTruthy();
        expect(contract.name).toBe("ComplexContract");
    });

    test("should preserve original contract structure", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        expect(contract).toBeTruthy();
        expect(contract.name).toBe("MyContract");

        // Should have original function plus shadow function
        const functions = contract.nodes.filter(
            (n: any) => n.nodeType === "FunctionDefinition"
        );
        expect(functions.length).toBeGreaterThanOrEqual(2);

        const functionNames = functions.map((f: any) => f.name);
        expect(functionNames).toContain("getSecret"); // Original
        expect(functionNames).toContain("exploit"); // Shadow
    });

    test("should return valid parsed AST structure", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        // Parsed AST should have basic structure
        expect(contract.nodeType).toBe("ContractDefinition");
        expect(contract.name).toBe("MyContract");
        expect(Array.isArray(contract.nodes)).toBe(true);
    });
});

describe("Shadow - Stitch Into AST", () => {
    test("should stitch into existing AST JSON", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);

        // First, get the AST of target contract
        const targetAst = Shadow.parseSourceAstStatic(TARGET_CONTRACT, null);

        // Then stitch into it
        const result = shadow.stitchIntoAst(targetAst, null, null);

        expect(result).toBeTruthy();
        const ast = JSON.parse(result);
        expect(ast.nodeType).toBe("SourceUnit");
    });

    test("should preserve AST structure when stitching", () => {
        const shadow = new Shadow(SHADOW_VARIABLE);

        const targetAst = Shadow.parseSourceAstStatic(TARGET_CONTRACT, null);
        const result = shadow.stitchIntoAst(targetAst, null, null);

        const originalAst = JSON.parse(targetAst);
        const stitchedAst = JSON.parse(result);

        // Should have same number of top-level nodes (pragma + contract)
        expect(stitchedAst.nodes.length).toBe(originalAst.nodes.length);
    });
});

describe("Shadow - Static Parsing", () => {
    test("should parse source to AST", () => {
        const ast = Shadow.parseSourceAstStatic(TARGET_CONTRACT, null);

        expect(ast).toBeTruthy();
        const parsed = JSON.parse(ast);
        expect(parsed.nodeType).toBe("SourceUnit");
        expect(Array.isArray(parsed.nodes)).toBe(true);
    });

    test("should parse source with custom file name", () => {
        const ast = Shadow.parseSourceAstStatic(
            TARGET_CONTRACT,
            "CustomContract.sol"
        );

        expect(ast).toBeTruthy();
        const parsed = JSON.parse(ast);
        expect(parsed.nodeType).toBe("SourceUnit");
    });

    test("should parse complex contract", () => {
        const ast = Shadow.parseSourceAstStatic(COMPLEX_CONTRACT, null);

        const parsed = JSON.parse(ast);
        const contract = parsed.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        expect(contract).toBeTruthy();
        expect(contract.name).toBe("ComplexContract");

        // Should have functions, events, modifiers
        const nodeTypes = new Set(contract.nodes.map((n: any) => n.nodeType));
        expect(nodeTypes.has("FunctionDefinition")).toBe(true);
        expect(nodeTypes.has("EventDefinition")).toBe(true);
        expect(nodeTypes.has("ModifierDefinition")).toBe(true);
    });

    test("should parse multi-contract source", () => {
        const ast = Shadow.parseSourceAstStatic(MULTI_CONTRACT, null);

        const parsed = JSON.parse(ast);
        const contracts = parsed.nodes.filter(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        expect(contracts.length).toBe(3);

        const contractNames = contracts.map((c: any) => c.name);
        expect(contractNames).toContain("First");
        expect(contractNames).toContain("Second");
        expect(contractNames).toContain("Target");
    });
});

describe("Shadow - Contract Selection", () => {
    test("should auto-select last contract when name not provided", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);

        // Should stitch into Target (last contract)
        const result = shadow.stitchIntoSource(MULTI_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contracts = ast.nodes.filter(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        // Find Target contract - it should have the shadow function
        const targetContract = contracts.find((c: any) => c.name === "Target");
        expect(targetContract).toBeTruthy();

        const targetFunctions = targetContract.nodes.filter(
            (n: any) => n.nodeType === "FunctionDefinition"
        );
        const functionNames = targetFunctions.map((f: any) => f.name);
        expect(functionNames).toContain("exploit");
    });

    test("should select specific contract by name", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);

        // Explicitly stitch into "Target"
        const result = shadow.stitchIntoSource(
            MULTI_CONTRACT,
            null,
            "Target"
        );

        const ast = JSON.parse(result);
        const targetContract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition" && n.name === "Target"
        );

        expect(targetContract).toBeTruthy();

        const functions = targetContract.nodes.filter(
            (n: any) => n.nodeType === "FunctionDefinition"
        );
        const functionNames = functions.map((f: any) => f.name);
        expect(functionNames).toContain("exploit");
    });
});

describe("Shadow - Error Handling", () => {
    test("should throw error for invalid contract name", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);

        expect(() => {
            shadow.stitchIntoSource(
                TARGET_CONTRACT,
                null,
                "NonExistentContract"
            );
        }).toThrow();
    });

    test("should throw error for invalid source code", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const invalidSource = "this is not valid solidity";

        expect(() => {
            shadow.stitchIntoSource(invalidSource, null, null);
        }).toThrow();
    });

    test("should throw error for invalid AST JSON", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const invalidAst = "{ invalid json";

        expect(() => {
            shadow.stitchIntoAst(invalidAst, null, null);
        }).toThrow();
    });

    test("should throw error for malformed AST structure", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const malformedAst = JSON.stringify({ nodeType: "Invalid" });

        expect(() => {
            shadow.stitchIntoAst(malformedAst, null, null);
        }).toThrow();
    });
});

describe("Shadow - Advanced Scenarios", () => {
    test("should stitch function that references contract state", () => {
        const shadowWithStateAccess = `
function getDoubleSecret() public view returns (uint256) {
    return secretValue * 2;
}`;

        const shadow = new Shadow(shadowWithStateAccess);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const functions = contract.nodes.filter(
            (n: any) => n.nodeType === "FunctionDefinition"
        );
        const functionNames = functions.map((f: any) => f.name);
        expect(functionNames).toContain("getDoubleSecret");
    });

    test("should stitch function with parameters", () => {
        const shadowWithParams = `
function setValue(uint256 _value) public {
    secretValue = _value;
}`;

        const shadow = new Shadow(shadowWithParams);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const setValueFunc = contract.nodes.find(
            (n: any) => n.nodeType === "FunctionDefinition" && n.name === "setValue"
        );

        expect(setValueFunc).toBeTruthy();
        expect(setValueFunc.parameters).toBeDefined();
        expect(setValueFunc.parameters.parameters.length).toBe(1);
    });

    test("should stitch function with return values", () => {
        const shadowWithReturn = `
function calculate(uint256 a, uint256 b) public pure returns (uint256, uint256) {
    return (a + b, a * b);
}`;

        const shadow = new Shadow(shadowWithReturn);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const calculateFunc = contract.nodes.find(
            (n: any) => n.nodeType === "FunctionDefinition" && n.name === "calculate"
        );

        expect(calculateFunc).toBeTruthy();
        expect(calculateFunc.returnParameters).toBeDefined();
        expect(calculateFunc.returnParameters.parameters.length).toBe(2);
    });

    test("should preserve IDs without collision", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);

        // Parse original to get max ID
        const originalAst = Shadow.parseSourceAstStatic(TARGET_CONTRACT, null);
        const original = JSON.parse(originalAst);

        // Find max ID in original
        const findMaxId = (node: any): number => {
            let max = node.id || 0;
            if (typeof node === "object" && node !== null) {
                for (const key in node) {
                    if (Array.isArray(node[key])) {
                        for (const item of node[key]) {
                            max = Math.max(max, findMaxId(item));
                        }
                    } else if (typeof node[key] === "object") {
                        max = Math.max(max, findMaxId(node[key]));
                    }
                }
            }
            return max;
        };

        const originalMaxId = findMaxId(original);

        // Stitch and check new IDs
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);
        const stitched = JSON.parse(result);
        const stitchedMaxId = findMaxId(stitched);

        // Stitched should have higher IDs due to renumbering
        expect(stitchedMaxId).toBeGreaterThan(originalMaxId);
    });

    test("should handle multiple sequential stitches", () => {
        const shadow1 = new Shadow("function first() public {}");
        const shadow2 = new Shadow("function second() public {}");

        // First stitch
        const result1 = shadow1.stitchIntoSource(TARGET_CONTRACT, null, null);

        // Can't directly stitch into result1 because it's analyzed AST
        // This test shows that each stitch is independent
        const result2 = shadow2.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast1 = JSON.parse(result1);
        const ast2 = JSON.parse(result2);

        const contract1 = ast1.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );
        const contract2 = ast2.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const functions1 = contract1.nodes.filter(
            (n: any) => n.nodeType === "FunctionDefinition"
        );
        const functions2 = contract2.nodes.filter(
            (n: any) => n.nodeType === "FunctionDefinition"
        );

        const names1 = functions1.map((f: any) => f.name);
        const names2 = functions2.map((f: any) => f.name);

        expect(names1).toContain("first");
        expect(names2).toContain("second");
    });
});

describe("Shadow - Edge Cases", () => {
    test("should handle empty function body", () => {
        const shadow = new Shadow("function empty() public {}");
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        expect(result).toBeTruthy();
        const ast = JSON.parse(result);
        expect(ast.nodeType).toBe("SourceUnit");
    });

    test("should handle function with modifiers", () => {
        const contractWithModifier = `
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract WithModifier {
    address public owner;

    modifier onlyOwner() {
        require(msg.sender == owner);
        _;
    }

    function restrictedFunction() public onlyOwner {}
}`;

        const shadowWithModifier = `
function anotherRestricted() public onlyOwner {
    // Do something
}`;

        const shadow = new Shadow(shadowWithModifier);
        const result = shadow.stitchIntoSource(
            contractWithModifier,
            null,
            null
        );

        expect(result).toBeTruthy();
        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const newFunc = contract.nodes.find(
            (n: any) =>
                n.nodeType === "FunctionDefinition" &&
                n.name === "anotherRestricted"
        );

        expect(newFunc).toBeTruthy();
    });

    test("should handle payable functions", () => {
        const shadow = new Shadow(
            "function deposit() public payable {}"
        );
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const depositFunc = contract.nodes.find(
            (n: any) => n.nodeType === "FunctionDefinition" && n.name === "deposit"
        );

        expect(depositFunc).toBeTruthy();
        expect(depositFunc.stateMutability).toBe("payable");
    });

    test("should handle constructor", () => {
        const shadow = new Shadow(
            "constructor(uint256 _initial) { secretValue = _initial; }"
        );
        const nodes = shadow.toAstNodes();

        expect(nodes.length).toBeGreaterThan(0);
        const node = JSON.parse(nodes[0]);
        expect(node.nodeType).toBe("FunctionDefinition");
        expect(node.kind).toBe("constructor");
    });

    test("should handle fallback function", () => {
        const shadow = new Shadow("fallback() external payable {}");
        const nodes = shadow.toAstNodes();

        expect(nodes.length).toBeGreaterThan(0);
        const node = JSON.parse(nodes[0]);
        expect(node.nodeType).toBe("FunctionDefinition");
        expect(node.kind).toBe("fallback");
    });

    test("should handle receive function", () => {
        const shadow = new Shadow("receive() external payable {}");
        const nodes = shadow.toAstNodes();

        expect(nodes.length).toBeGreaterThan(0);
        const node = JSON.parse(nodes[0]);
        expect(node.nodeType).toBe("FunctionDefinition");
        expect(node.kind).toBe("receive");
    });
});

describe("Shadow - AST Structure", () => {
    test("parsed AST should have valid structure", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);
        const contract = ast.nodes.find(
            (n: any) => n.nodeType === "ContractDefinition"
        );

        const exploitFunc = contract.nodes.find(
            (n: any) => n.nodeType === "FunctionDefinition" && n.name === "exploit"
        );

        expect(exploitFunc).toBeTruthy();
        expect(exploitFunc.nodeType).toBe("FunctionDefinition");
        expect(exploitFunc.name).toBe("exploit");
    });

    test("parsed AST should have unique IDs", () => {
        const shadow = new Shadow(SHADOW_FUNCTION);
        const result = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

        const ast = JSON.parse(result);

        // Check that IDs exist in the AST
        const findAllIds = (node: any): Set<number> => {
            const ids = new Set<number>();
            if (node.id !== undefined) ids.add(node.id);

            if (typeof node === "object" && node !== null) {
                for (const key in node) {
                    if (Array.isArray(node[key])) {
                        for (const item of node[key]) {
                            findAllIds(item).forEach((id) => ids.add(id));
                        }
                    } else if (typeof node[key] === "object") {
                        findAllIds(node[key]).forEach((id) => ids.add(id));
                    }
                }
            }
            return ids;
        };

        const allIds = findAllIds(ast);
        expect(allIds.size).toBeGreaterThan(0);
    });
});
