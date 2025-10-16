import { beforeAll, describe, expect, test } from "bun:test";
import { readFileSync } from "fs";
import { join } from "path";
import { Compiler, Shadow } from "../build/index.js";
import type { ContractDefinition, SourceUnit } from "../build/ast-types.js";

const DEFAULT_SOLC_VERSION = "0.8.30";
const AST_FIXTURES_DIR = join(__dirname, "fixtures", "ast");
let sharedCompiler: Compiler;

const TARGET_CONTRACT = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MyContract {
  uint256 private secretValue;

  function getSecret() public view returns (uint256) {
    return secretValue;
  }
}
`;

const MULTI_CONTRACT = `// SPDX-License-Identifier: MIT
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

const SHADOW_FUNCTION = `function tapSecret() public view returns (uint256) {
  return secretValue + 1;
}`;

const SHADOW_VARIABLE = `uint256 public exposed;`;

beforeAll(() => {
  if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
    throw new Error(
      `Solc ${DEFAULT_SOLC_VERSION} must be installed before running shadow tests. ` +
        `Install it via Compiler.installSolcVersion or Foundry's svm ahead of time.`,
    );
  }
  sharedCompiler = new Compiler({
    solcVersion: DEFAULT_SOLC_VERSION,
  });
});

const loadAst = (filename: string): SourceUnit =>
  JSON.parse(readFileSync(join(AST_FIXTURES_DIR, filename), "utf8")) as SourceUnit;

const findContract = (ast: SourceUnit, name: string): ContractDefinition | undefined =>
  ast.nodes
    .filter((node) => node.nodeType === "ContractDefinition")
    .find((definition) => definition.name === name);

describe("Shadow instances", () => {
  test("can be constructed directly without compiler helper", () => {
    const direct = new Shadow(SHADOW_FUNCTION, {
      solcVersion: DEFAULT_SOLC_VERSION,
    });
    const stitched = direct.stitchIntoSource(TARGET_CONTRACT);
    const contract = findContract(stitched, "MyContract");
    expect(contract).toBeTruthy();
  });

  test("rejects malformed settings objects", () => {
    expect(() => new Shadow(SHADOW_FUNCTION, { settings: 42 as any })).toThrow(/settings override must be provided/i);
  });

  test("inherits compiler defaults when created via helper", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    const stitched = shadow.stitchIntoSource(TARGET_CONTRACT);
    const contract = findContract(stitched, "MyContract");
    expect(contract).toBeTruthy();
  });

  test("sanitises constructor-provided settings and stitches functions", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION, {
      solcVersion: DEFAULT_SOLC_VERSION,
      settings: { stopAfter: "parsing" },
    });
    const stitched = shadow.stitchIntoSource(TARGET_CONTRACT);
    const contract = findContract(stitched, "MyContract");

    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);

    expect(functionNames).toEqual(expect.arrayContaining(["getSecret", "tapSecret"]));
  });

  test("allows per-call overrides while still producing AST output", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_VARIABLE);
    const stitched = shadow.stitchIntoSource(MULTI_CONTRACT, undefined, "Target", {
      settings: { stopAfter: "parsing" },
    });

    const contract = findContract(stitched, "Target");
    expect(contract).toBeTruthy();
    const nodeTypes = contract!.nodes.map((node) => node.nodeType);
    expect(nodeTypes).toContain("VariableDeclaration");
  });

  test("throws when per-call overrides reference unknown solc versions", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    expect(() =>
      shadow.stitchIntoSource(TARGET_CONTRACT, undefined, undefined, {
        solcVersion: "123.45.67",
      }),
    ).toThrow(/not installed/i);
  });

  test("fails early when helper is asked to create a shadow with missing solc version", () => {
    expect(() =>
      sharedCompiler.createShadow(SHADOW_FUNCTION, {
        solcVersion: "123.45.67",
      }),
    ).toThrow(/not installed/i);
  });
});

describe("Shadow.stitchIntoAst", () => {
  const collectIds = (node: unknown, target: number[]) => {
    if (Array.isArray(node)) {
      node.forEach((child) => collectIds(child, target));
      return;
    }
    if (node && typeof node === "object") {
      const value = node as Record<string, unknown>;
      if (typeof value.id === "number") {
        target.push(value.id);
      }
      Object.values(value).forEach((child) => collectIds(child, target));
    }
  };

  test("injects shadow members without mutating the original AST reference", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    const targetAst = loadAst("shadow-target-ast.json");
    const originalSnapshot = JSON.parse(JSON.stringify(targetAst));

    const stitched = shadow.stitchIntoAst(targetAst);
    const contract = findContract(stitched, "MyContract");
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);

    expect(functionNames).toContain("tapSecret");
    expect(targetAst).toEqual(originalSnapshot);
  });

  test("defaults to the last contract in multi-contract files", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    const stitched = shadow.stitchIntoSource(MULTI_CONTRACT);
    const target = findContract(stitched, "Target");
    const first = findContract(stitched, "First");

    expect(target).toBeTruthy();
    expect(first?.nodes.some((node) => node.nodeType === "FunctionDefinition" && (node as any).name === "tapSecret")).toBe(
      false,
    );
  });

  test("ensures ids remain unique after stitching", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    const targetAst = loadAst("shadow-target-ast.json");
    const stitched = shadow.stitchIntoAst(targetAst);
    const ids: number[] = [];
    collectIds(stitched, ids);
    const unique = new Set(ids);

    expect(ids.length).toBe(unique.size);
    expect(ids.length).toBeGreaterThan(0);
  });

  test("targets specific contract definitions in multi-contract sources", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    const targetAst = loadAst("shadow-multi-ast.json");
    const stitched = shadow.stitchIntoAst(targetAst, "Target");

    const target = findContract(stitched, "Target");
    expect(target).toBeTruthy();

    const other = findContract(stitched, "First");
    const targetFnNames = target!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);
    const otherFnNames = other!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);

    expect(targetFnNames).toContain("tapSecret");
    expect(otherFnNames).not.toContain("tapSecret");
  });
});

describe("Shadow error handling", () => {
  test("throws when contract name cannot be found", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    expect(() => shadow.stitchIntoSource(TARGET_CONTRACT, undefined, "Missing")).toThrow(/Contract 'Missing'/i);
  });

  test("throws when provided Solidity is invalid", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    expect(() => shadow.stitchIntoSource("contract {")).toThrow(/Failed to parse target source/i);
  });

  test("throws when AST input is malformed", () => {
    const shadow = sharedCompiler.createShadow(SHADOW_FUNCTION);
    expect(() => shadow.stitchIntoAst({ not: "an ast" } as any)).toThrow(/Failed to locate target contract/i);
  });
});
