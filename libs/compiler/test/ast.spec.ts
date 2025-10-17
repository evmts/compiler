import { beforeAll, describe, expect, test } from "bun:test";
import { Ast, Compiler } from "../build/index.js";
import type { ContractDefinition, SourceUnit } from "../build/ast-types.js";

const DEFAULT_SOLC_VERSION = "0.8.30";
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
  function internalOnly() internal view returns (uint256) {
    return secret;
  }
}
`;

const AST_FUNCTION = `function tapSecret() public view returns (uint256) {
  return secretValue + 1;
}`;

const AST_VARIABLE = `uint256 public exposed;`;

beforeAll(() => {
  if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
    throw new Error(
      `Solc ${DEFAULT_SOLC_VERSION} must be installed before running ast tests. ` +
        `Install it via Compiler.installSolcVersion or Foundry's svm ahead of time.`
    );
  }
  sharedCompiler = new Compiler({ solcVersion: DEFAULT_SOLC_VERSION });
});

const createAst = () => new Ast({ solcVersion: DEFAULT_SOLC_VERSION });

const findContract = (
  ast: SourceUnit,
  name: string
): ContractDefinition | undefined =>
  ast.nodes
    .filter((node) => node.nodeType === "ContractDefinition")
    .find((definition) => definition.name === name);

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

describe("Ast construction", () => {
  test("hydrates from source using standalone constructor", () => {
    const instrumented = createAst()
      .fromSource(TARGET_CONTRACT)
      .injectShadow(AST_FUNCTION);
    const contract = findContract(instrumented.ast(), "MyContract");
    expect(contract).toBeTruthy();
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);
    expect(functionNames).toContain("tapSecret");
  });

  test("rejects malformed settings objects", () => {
    expect(() => new Ast({ settings: 42 as unknown as any })).toThrow(
      /settings override must be provided/i
    );
  });

  test("hydrates from typed ast values", () => {
    const sourceAst = createAst().fromSource(TARGET_CONTRACT).ast();
    expect(createAst().fromSource(sourceAst).ast()).toEqual(sourceAst);
  });
});

describe("Ast operations", () => {
  test("injectShadow merges prebuilt ast fragments", () => {
    const fragment = createAst()
      .fromSource(
        `// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

contract __AstFragment {
  ${AST_FUNCTION}
}
`
      )
      .ast();

    const instrumented = createAst()
      .fromSource(TARGET_CONTRACT)
      .injectShadow(fragment);

    const contract = findContract(instrumented.ast(), "MyContract");
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);
    expect(functionNames).toContain("tapSecret");
  });

  test("exposeInternalVariables promotes visibility", () => {
    const instrumented = createAst()
      .fromSource(MULTI_CONTRACT, { instrumentedContract: "Target" })
      .exposeInternalVariables({ instrumentedContract: "Target" });

    const target = findContract(instrumented.ast(), "Target")!;
    const visibility = target.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node) => (node as any).visibility);
    expect(visibility).toContain("public");

    const first = findContract(instrumented.ast(), "First")!;
    const firstVisibility = first.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node) => (node as any).visibility);
    expect(new Set(firstVisibility)).toEqual(new Set(["public"]));
  });

  test("exposeInternalFunctions promotes visibility", () => {
    const instrumented = createAst()
      .fromSource(MULTI_CONTRACT, { instrumentedContract: "Target" })
      .exposeInternalFunctions({ instrumentedContract: "Target" });

    const target = findContract(instrumented.ast(), "Target")!;
    const functionVis = target.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((node) => (node as any).visibility);
    expect(functionVis).toContain("public");
  });

  test("ids remain unique after sequential injections", () => {
    const instrumented = createAst()
      .fromSource(TARGET_CONTRACT)
      .injectShadow(AST_FUNCTION)
      .injectShadow(AST_VARIABLE);

    const ast = instrumented.ast();
    const ids: number[] = [];
    collectIds(ast, ids);
    expect(ids.length).toBeGreaterThan(0);
    expect(ids.length).toBe(new Set(ids).size);
  });

  test("transformed ast compiles successfully", () => {
    const instrumented = createAst()
      .fromSource(TARGET_CONTRACT)
      .injectShadow(AST_FUNCTION)
      .injectShadow(AST_VARIABLE);

    const ast = instrumented.ast();
    const output = sharedCompiler.compileSource(ast);
    expect(output.hasCompilerErrors).toBe(false);
  });
});
