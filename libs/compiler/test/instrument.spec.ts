import { beforeAll, describe, expect, test } from "bun:test";
import { Compiler, Instrument } from "../build/index.js";
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

const INSTRUMENT_FUNCTION = `function tapSecret() public view returns (uint256) {
  return secretValue + 1;
}`;

const INSTRUMENT_VARIABLE = `uint256 public exposed;`;

beforeAll(() => {
  if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
    throw new Error(
      `Solc ${DEFAULT_SOLC_VERSION} must be installed before running instrument tests. ` +
        `Install it via Compiler.installSolcVersion or Foundry's svm ahead of time.`,
    );
  }
  sharedCompiler = new Compiler({
    solcVersion: DEFAULT_SOLC_VERSION,
  });
});

const findContract = (ast: SourceUnit, name: string): ContractDefinition | undefined =>
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

describe("Instrument construction", () => {
  test("hydrates from source using standalone constructor", () => {
    const instrument = new Instrument({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(
      TARGET_CONTRACT,
    );
    const injected = instrument.injectShadowSource(INSTRUMENT_FUNCTION);
    const contract = findContract(injected.ast(), "MyContract");
    expect(contract).toBeTruthy();
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);
    expect(functionNames).toContain("tapSecret");
  });

  test("rejects malformed settings objects", () => {
    expect(() => new Instrument({ settings: 42 as unknown as any })).toThrow(
      /settings override must be provided/i,
    );
  });

  test("hydrates from typed ast values", () => {
    const ast = sharedCompiler.instrumentFromSource(TARGET_CONTRACT).ast()
    const instrument = new Instrument({ solcVersion: DEFAULT_SOLC_VERSION }).fromAst(ast);
    expect(instrument.ast()).toEqual(ast);
  });
});

describe("Compiler instrumentation helpers", () => {
  test("instrumentFromSource shares compiler defaults", () => {
    const instrument = sharedCompiler
      .instrumentFromSource(TARGET_CONTRACT)
      .injectShadowSource(INSTRUMENT_FUNCTION);
    const contract = findContract(instrument.ast(), "MyContract");
    expect(contract).toBeTruthy();
  });

  test("instrumentFromAst preserves input snapshot", () => {
    const targetAst = sharedCompiler.instrumentFromSource(TARGET_CONTRACT).ast()
    const snapshot = JSON.parse(JSON.stringify(targetAst));
    const instrument = sharedCompiler
      .instrumentFromAst(targetAst)
      .injectShadowSource(INSTRUMENT_FUNCTION);

    expect(findContract(instrument.ast(), "MyContract")).toBeTruthy();
    expect(targetAst).toEqual(snapshot);
  });

  test("standalone Instrument provides sanitised defaults", () => {
    const instrument = new Instrument({ solcVersion: DEFAULT_SOLC_VERSION })
      .fromSource(TARGET_CONTRACT)
      .injectShadowSource(INSTRUMENT_VARIABLE);
    const contract = findContract(instrument.ast(), "MyContract");
    expect(contract).toBeTruthy();
    const nodeTypes = contract!.nodes.map((node) => node.nodeType);
    expect(nodeTypes).toContain("VariableDeclaration");
  });
});

describe("Instrumentation operations", () => {
  test("injectShadowAst merges prebuilt ast fragments", () => {
    const fragment = new Instrument({ solcVersion: DEFAULT_SOLC_VERSION })
      .fromSource(`// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

contract __InstrumentFragment {
  ${INSTRUMENT_FUNCTION}
}
`)
      .ast();

    const instrument = sharedCompiler
      .instrumentFromSource(TARGET_CONTRACT)
      .injectShadowAst(fragment);

    const contract = findContract(instrument.ast(), "MyContract");
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn) => fn.name);
    expect(functionNames).toContain("tapSecret");
  });

  test("exposeInternalVariables promotes visibility", () => {
    const instrument = sharedCompiler
      .instrumentFromSource(MULTI_CONTRACT, { instrumentedContract: "Target" })
      .exposeInternalVariables({ instrumentedContract: "Target" });

    const target = findContract(instrument.ast(), "Target")!;
    const visibility = target.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node) => (node as any).visibility);
    expect(visibility).toContain("public");

    const first = findContract(instrument.ast(), "First")!;
    const firstVisibility = first.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node) => (node as any).visibility);
    expect(new Set(firstVisibility)).toEqual(new Set(["public"]));
  });

  test("exposeInternalFunctions promotes visibility", () => {
    const instrument = sharedCompiler
      .instrumentFromSource(MULTI_CONTRACT, { instrumentedContract: "Target" })
      .exposeInternalFunctions({ instrumentedContract: "Target" });

    const target = findContract(instrument.ast(), "Target")!;
    const functionVis = target.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((node) => (node as any).visibility);
    expect(functionVis).toContain("public");
  });

  test("ids remain unique after sequential injections", () => {
    const instrument = sharedCompiler
      .instrumentFromSource(TARGET_CONTRACT)
      .injectShadowSource(INSTRUMENT_FUNCTION)
      .injectShadowSource(INSTRUMENT_VARIABLE);

    const ast = instrument.ast();
    const ids: number[] = [];
    collectIds(ast, ids);
    expect(ids.length).toBeGreaterThan(0);
    expect(ids.length).toBe(new Set(ids).size);
  });

  test("instrumented ast compiles successfully", () => {
    const ast = sharedCompiler
      .instrumentFromSource(TARGET_CONTRACT)
      .injectShadowSource(INSTRUMENT_FUNCTION)
      .exposeInternalVariables()
      .exposeInternalFunctions()
      .ast();

    const output = sharedCompiler.compileAst(ast);
    expect(output.hasCompilerErrors).toBe(false);
    expect(output.artifacts).not.toHaveLength(0);
  });
});

describe("Instrument error handling", () => {
  test("throws when contract name cannot be found", () => {
    const instrument = sharedCompiler.instrumentFromSource(TARGET_CONTRACT);
    expect(() =>
      instrument.injectShadowSource(INSTRUMENT_FUNCTION, { instrumentedContract: "Missing" }),
    ).toThrow(/Contract 'Missing'/i);
  });

  test("throws when provided Solidity is invalid", () => {
    const instrument = new Instrument({ solcVersion: DEFAULT_SOLC_VERSION });
    expect(() => instrument.fromSource("contract {")).toThrow(/Failed to parse target source/i);
  });

  test("throws when AST input is malformed", () => {
    const instrument = new Instrument({ solcVersion: DEFAULT_SOLC_VERSION });
    expect(() => instrument.fromAst({ not: "an ast" } as any)).toThrow(/missing field `id`/i);
  });
});
