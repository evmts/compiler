import { beforeAll, describe, expect, test } from "bun:test";
import { readFileSync } from "fs";
import { join } from "path";
import { Ast, Compiler } from "../build/index.js";
import type { ContractDefinition, SourceUnit } from "../build/ast-types.js";

const DEFAULT_SOLC_VERSION = "0.8.30";
const FIXTURES_DIR = join(__dirname, "fixtures");
const CONTRACTS_DIR = join(FIXTURES_DIR, "contracts");
const FRAGMENTS_DIR = join(FIXTURES_DIR, "fragments");
const AST_DIR = join(FIXTURES_DIR, "ast");

const INLINE_SOURCE = readFileSync(
  join(CONTRACTS_DIR, "InlineExample.sol"),
  "utf8"
);
const MULTI_CONTRACT_SOURCE = readFileSync(
  join(CONTRACTS_DIR, "MultiContract.sol"),
  "utf8"
);
const NO_CONTRACTS_SOURCE = readFileSync(
  join(CONTRACTS_DIR, "NoContracts.sol"),
  "utf8"
);
const FUNCTION_FRAGMENT = readFileSync(
  join(FRAGMENTS_DIR, "function_fragment.sol"),
  "utf8"
);
const VARIABLE_FRAGMENT = readFileSync(
  join(FRAGMENTS_DIR, "variable_fragment.sol"),
  "utf8"
);
const SHADOW_CONTRACT_FRAGMENT = readFileSync(
  join(FRAGMENTS_DIR, "shadow_contract.sol"),
  "utf8"
);
const EMPTY_SOURCE_UNIT = JSON.parse(
  readFileSync(join(AST_DIR, "empty_source_unit.json"), "utf8")
) as SourceUnit;
const FRAGMENT_WITHOUT_TARGET = JSON.parse(
  readFileSync(join(AST_DIR, "fragment_without_contract.json"), "utf8")
) as SourceUnit;

let sharedCompiler: Compiler;

const createAst = (options?: ConstructorParameters<typeof Ast>[0]) =>
  new Ast({ solcVersion: DEFAULT_SOLC_VERSION, ...options });

const findContract = (
  unit: SourceUnit,
  name: string
): ContractDefinition | undefined =>
  unit.nodes
    .filter((node) => node.nodeType === "ContractDefinition")
    .map((node) => node as unknown as ContractDefinition)
    .find((definition) => definition.name === name);

const collectIds = (value: unknown, ids: number[]) => {
  if (Array.isArray(value)) {
    value.forEach((child) => collectIds(child, ids));
    return;
  }
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (typeof record.id === "number") {
      ids.push(record.id);
    }
    Object.values(record).forEach((child) => collectIds(child, ids));
  }
};

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

beforeAll(() => {
  if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
    throw new Error(
      `Solc ${DEFAULT_SOLC_VERSION} must be installed before running ast tests. ` +
        `Install it via Compiler.installSolcVersion or Foundry's svm ahead of time.`
    );
  }
  sharedCompiler = new Compiler({ solcVersion: DEFAULT_SOLC_VERSION });
});

describe("Ast constructor", () => {
  test("creates instances with default configuration", () => {
    const ast = new Ast();
    expect(ast).toBeInstanceOf(Ast);
  });

  test("rejects malformed settings objects", () => {
    expect(() => new Ast({ settings: 42 as unknown as any })).toThrow(
      /settings override must be provided/i
    );
  });

  test("rejects unsupported solc language overrides", () => {
    expect(() => new Ast({ solcLanguage: "Yul" as any })).toThrow(
      /Ast helpers only support/i
    );
  });

  test("rejects when requested solc version is not installed", () => {
    expect(() => new Ast({ solcVersion: "999.0.0" })).toThrow(/not installed/i);
  });
});

describe("fromSource", () => {
  test("hydrates from source string", () => {
    const instrumented = createAst().fromSource(INLINE_SOURCE);
    const ast = instrumented.ast();

    const contract = findContract(ast as SourceUnit, "InlineExample");
    expect(contract).toBeTruthy();
  });

  test("hydrates from existing ast values", () => {
    const sourceAst = createAst().fromSource(INLINE_SOURCE).ast();
    const roundTripped = createAst().fromSource(sourceAst).ast();
    expect(roundTripped).toEqual(sourceAst);
  });

  test("applies instrumentedContract overrides per call", () => {
    const instrumented = createAst({
      instrumentedContract: "Target",
    }).fromSource(MULTI_CONTRACT_SOURCE);
    const ast = instrumented.ast() as SourceUnit;
    const target = findContract(ast, "Target");
    const second = findContract(ast, "Second");

    expect(target).toBeTruthy();
    expect(second).toBeTruthy();
  });

  test("throws when ast is requested before initialization", () => {
    const ast = createAst();
    expect(() => ast.ast()).toThrow(/Call fromSource first/i);
  });

  test("handles missing contracts when instrumented contract is configured", () => {
    const ast = createAst({ instrumentedContract: "Missing" }).fromSource(
      NO_CONTRACTS_SOURCE
    );
    const unit = ast.ast() as SourceUnit;
    const contracts = unit.nodes.filter(
      (node) => node.nodeType === "ContractDefinition"
    );
    expect(contracts).toHaveLength(0);
  });
});

describe("injectShadow", () => {
  test("injects fragment functions from source strings", () => {
    const instrumented = createAst()
      .fromSource(INLINE_SOURCE)
      .injectShadow(FUNCTION_FRAGMENT);
    const contract = findContract(
      instrumented.ast() as SourceUnit,
      "InlineExample"
    );
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn: any) => fn.name);
    expect(functionNames).toContain("tapStored");
  });

  test("injects fragment variables sequentially and keeps ids unique", () => {
    const instrumented = createAst()
      .fromSource(INLINE_SOURCE)
      .injectShadow(FUNCTION_FRAGMENT)
      .injectShadow(VARIABLE_FRAGMENT);

    const ast = instrumented.ast();
    const ids: number[] = [];
    collectIds(ast, ids);
    expect(ids.length).toBeGreaterThan(0);
    expect(ids.length).toBe(new Set(ids).size);
  });

  test("injects pre-parsed ast fragments", () => {
    const fragmentAst = createAst().fromSource(SHADOW_CONTRACT_FRAGMENT).ast();
    const instrumented = createAst()
      .fromSource(INLINE_SOURCE)
      .injectShadow(fragmentAst);
    const contract = findContract(
      instrumented.ast() as SourceUnit,
      "InlineExample"
    );
    const functionNames = contract!.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((fn: any) => fn.name);
    expect(functionNames).toContain("shadowy");
  });

  test("rejects fragments without __AstFragment contract", () => {
    const ast = createAst().fromSource(INLINE_SOURCE);
    expect(() => ast.injectShadow(clone(FRAGMENT_WITHOUT_TARGET))).toThrow();
  });

  test("rejects injection before loading a source", () => {
    const ast = createAst();
    expect(() => ast.injectShadow(FUNCTION_FRAGMENT)).toThrow(
      /Call fromSource first/i
    );
  });
});

describe("visibility transformations", () => {
  test("promotes private and internal variables to public", () => {
    const instrumented = createAst()
      .fromSource(MULTI_CONTRACT_SOURCE, { instrumentedContract: "Target" })
      .exposeInternalVariables({ instrumentedContract: "Target" });

    const target = findContract(instrumented.ast() as SourceUnit, "Target")!;
    const visibilities = target.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node: any) => node.visibility);
    expect(new Set(visibilities)).toEqual(new Set(["public"]));
  });

  test("promotes private and internal functions to public", () => {
    const instrumented = createAst()
      .fromSource(MULTI_CONTRACT_SOURCE, { instrumentedContract: "Target" })
      .exposeInternalFunctions({ instrumentedContract: "Target" });

    const target = findContract(instrumented.ast() as SourceUnit, "Target")!;
    const visibilities = target.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((node: any) => node.visibility);
    expect(visibilities).toContain("public");
  });

  test("applies visibility changes across all contracts when no override is provided", () => {
    const instrumented = createAst()
      .fromSource(MULTI_CONTRACT_SOURCE)
      .exposeInternalVariables()
      .exposeInternalFunctions();

    const ast = instrumented.ast() as SourceUnit;
    const first = findContract(ast, "First")!;
    const second = findContract(ast, "Second")!;
    const target = findContract(ast, "Target")!;

    const firstVars = first.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node: any) => node.visibility);
    const secondVars = second.nodes
      .filter((node) => node.nodeType === "VariableDeclaration")
      .map((node: any) => node.visibility);
    const targetFuncs = target.nodes
      .filter((node) => node.nodeType === "FunctionDefinition")
      .map((node: any) => node.visibility);

    expect(new Set(firstVars)).toEqual(new Set(["public"]));
    expect(new Set(secondVars)).toEqual(new Set(["public"]));
    expect(targetFuncs).toContain("public");
  });

  test("rejects visibility changes before loading a source", () => {
    const ast = createAst({ instrumentedContract: "Target" });
    expect(() => ast.exposeInternalVariables()).toThrow(
      /Call fromSource first/i
    );
    expect(() => ast.exposeInternalFunctions()).toThrow(
      /Call fromSource first/i
    );
  });

  test("throws when targeted contract is missing during visibility updates", () => {
    const instrumented = createAst().fromSource(MULTI_CONTRACT_SOURCE);
    expect(() =>
      instrumented.exposeInternalVariables({ instrumentedContract: "Missing" })
    ).toThrow(/Contract 'Missing' not found/i);
  });
});

describe("integration with Compiler", () => {
  test("compiled instrumented ast executes without diagnostics", () => {
    const instrumented = createAst()
      .fromSource(INLINE_SOURCE)
      .injectShadow(FUNCTION_FRAGMENT)
      .injectShadow(VARIABLE_FRAGMENT)
      .exposeInternalVariables()
      .exposeInternalFunctions();

    const ast = instrumented.ast();
    const output = sharedCompiler.compileSource(ast);

    expect(output.hasCompilerErrors).toBe(false);
    expect(output.artifacts[0].contractName).toBe("InlineExample");
  });

  test("handles ast inputs without contracts gracefully", () => {
    const output = sharedCompiler.compileSource(clone(EMPTY_SOURCE_UNIT));
    expect(output.artifacts).toHaveLength(0);
    expect(Array.isArray(output.errors)).toBe(true);
  });

  test("ast() returns sanitized json without null entries", () => {
    const ast = createAst()
      .fromSource(INLINE_SOURCE)
      .injectShadow(FUNCTION_FRAGMENT)
      .ast();
    const serialized = JSON.stringify(ast);
    expect(serialized.includes("null")).toBe(false);
  });
});
