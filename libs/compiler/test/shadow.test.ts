import { describe, expect, test } from "bun:test";
import { Shadow } from "../build/index.js";
import type { ContractDefinition, SourceUnit } from "../build/foundry-types.js";

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

const parseTargetAst = (source: string, fileName?: string): SourceUnit => {
  return Shadow.parseSourceAst(source, fileName ?? null);
};

const findContract = (
  ast: SourceUnit,
  name: string
): ContractDefinition | undefined => {
  const contracts = ast.nodes.filter(
    (part) => part.nodeType === "ContractDefinition"
  );
  return contracts.find((c) => c.name === name);
};

describe("Shadow - basic usage", () => {
  test("creates a Shadow instance", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);
    expect(shadow).toBeTruthy();
  });

  // Additional functionality covered in parse/stitch suites below.
});

describe("Shadow - parse source", () => {
  test("parses Solidity into typed AST", () => {
    const ast = parseTargetAst(TARGET_CONTRACT);

    expect(ast.nodes.length).toBeGreaterThan(0);
    const contract = findContract(ast, "MyContract");
    expect(contract).toBeTruthy();
    expect(contract?.nodes.length).toBeGreaterThan(0);
  });

  test("accepts custom file name", () => {
    const ast = parseTargetAst(TARGET_CONTRACT, "Custom.sol");
    expect(ast.absolutePath).toContain("Custom.sol");
  });
});

describe("Shadow - stitch into source", () => {
  test("injects shadow function into source", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);
    const ast = shadow.stitchIntoSource(TARGET_CONTRACT, null, null);

    const contract = findContract(ast, "MyContract");
    expect(contract).toBeTruthy();

    const functionNames = contract!.nodes
      .filter((part) => part.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);

    expect(functionNames).toContain("getSecret");
    expect(functionNames).toContain("exploit");
  });

  test("targets named contract", () => {
    const shadow = new Shadow(SHADOW_VARIABLE);
    const ast = shadow.stitchIntoSource(MULTI_CONTRACT, null, "Target");

    const target = findContract(ast, "Target");
    expect(target).toBeTruthy();

    const nodeTypes = new Set(target!.nodes.map((n) => n.nodeType));
    expect(nodeTypes.has("VariableDeclaration")).toBe(true);
  });

  test("stitches into complex contract", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);
    const ast = shadow.stitchIntoSource(COMPLEX_CONTRACT, null, null);

    const contract = findContract(ast, "ComplexContract");
    expect(contract).toBeTruthy();

    const nodeTypes = new Set(contract!.nodes.map((n) => n.nodeType));
    expect(nodeTypes.has("EventDefinition")).toBe(true);
    expect(nodeTypes.has("FunctionDefinition")).toBe(true);
  });
});

describe("Shadow - stitch into AST", () => {
  test("stitches into parsed AST object", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);
    const targetAst = parseTargetAst(TARGET_CONTRACT);

    const stitched = shadow.stitchIntoAst(targetAst, null, null);
    const contract = findContract(stitched, "MyContract");

    const functionNames = contract!.nodes
      .filter((part) => part.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);

    expect(functionNames).toContain("exploit");
  });

  test("selects specific contract when provided", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);
    const targetAst = parseTargetAst(MULTI_CONTRACT);

    const stitched = shadow.stitchIntoAst(targetAst, "Target", null);
    const target = findContract(stitched, "Target");
    expect(target).toBeTruthy();

    const functionNames = target!.nodes
      .filter((part) => part.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);

    expect(functionNames).toContain("exploit");
  });
});

describe("Shadow - error handling", () => {
  test("throws for invalid contract name", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);

    expect(() => {
      shadow.stitchIntoSource(TARGET_CONTRACT, null, "MissingContract");
    }).toThrow();
  });

  test("throws for invalid source", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);

    expect(() => {
      shadow.stitchIntoSource("not real solidity", null, null);
    }).toThrow();
  });

  test("throws for malformed AST object", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);
    const malformed = { nodeType: "SourceUnit" };

    expect(() => {
      shadow.stitchIntoAst(malformed, null, null);
    }).toThrow();
  });

  test("throws when AST input is not an object", () => {
    const shadow = new Shadow(SHADOW_FUNCTION);

    expect(() => {
      shadow.stitchIntoAst("not-an-ast", null, null);
    }).toThrow();
  });
});
