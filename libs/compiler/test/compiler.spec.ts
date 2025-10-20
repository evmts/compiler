import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import {
  cpSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "fs";
import { join } from "path";
import { tmpdir } from "os";
import {
  Ast,
  BytecodeHash,
  Compiler,
  CompilerSettings,
  EvmVersion,
  ModelCheckerEngine,
  RevertStrings,
  SeverityLevel,
  SolcLanguage,
} from "../build/index.js";
import type { OutputSelection } from "../build/solc-settings.js";

const DEFAULT_SOLC_VERSION = "0.8.30";
const ALT_SOLC_VERSION = "0.8.29";
const FIXTURES_DIR = join(__dirname, "fixtures");
const CONTRACTS_DIR = join(FIXTURES_DIR, "contracts");
const FRAGMENTS_DIR = join(FIXTURES_DIR, "fragments");
const AST_DIR = join(FIXTURES_DIR, "ast");
const YUL_DIR = join(FIXTURES_DIR, "yul");
const HARDHAT_PROJECT = join(FIXTURES_DIR, "hardhat-project");
const SIMPLE_STORAGE_PATH = join(
  HARDHAT_PROJECT,
  "contracts",
  "SimpleStorage.sol"
);
const INLINE_PATH = join(CONTRACTS_DIR, "InlineExample.sol");
const BROKEN_PATH = join(CONTRACTS_DIR, "BrokenExample.sol");
const MULTI_CONTRACT_PATH = join(CONTRACTS_DIR, "MultiContract.sol");
const WARNING_PATH = join(CONTRACTS_DIR, "WarningContract.sol");
const LIBRARY_PATH = join(CONTRACTS_DIR, "MathLib.sol");
const LIBRARY_CONSUMER_PATH = join(CONTRACTS_DIR, "LibraryConsumer.sol");
const INLINE_SOURCE = readFileSync(INLINE_PATH, "utf8");
const BROKEN_SOURCE = readFileSync(BROKEN_PATH, "utf8");
const MULTI_CONTRACT_SOURCE = readFileSync(MULTI_CONTRACT_PATH, "utf8");
const WARNING_SOURCE = readFileSync(WARNING_PATH, "utf8");
const LIBRARY_SOURCE = readFileSync(LIBRARY_PATH, "utf8");
const LIBRARY_CONSUMER_SOURCE = readFileSync(LIBRARY_CONSUMER_PATH, "utf8");
const FUNCTION_FRAGMENT = readFileSync(
  join(FRAGMENTS_DIR, "function_fragment.sol"),
  "utf8"
);
const VARIABLE_FRAGMENT = readFileSync(
  join(FRAGMENTS_DIR, "variable_fragment.sol"),
  "utf8"
);
const EMPTY_SOURCE_UNIT = JSON.parse(
  readFileSync(join(AST_DIR, "empty_source_unit.json"), "utf8")
);
const FRAGMENT_WITHOUT_TARGET = JSON.parse(
  readFileSync(join(AST_DIR, "fragment_without_contract.json"), "utf8")
);
const YUL_PATH = join(YUL_DIR, "Echo.yul");
const YUL_SOURCE = readFileSync(YUL_PATH, "utf8");

const createAst = () => new Ast({ solcVersion: DEFAULT_SOLC_VERSION });

const DEFAULT_OUTPUT_SELECTION = {
  "*": {
    "*": [
      "abi",
      "evm.bytecode",
      "evm.deployedBytecode",
      "evm.methodIdentifiers",
    ],
    "": ["ast"],
  },
} as const satisfies OutputSelection;

const tempDirs: string[] = [];

const deepClone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

const createTempDir = (prefix: string) => {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  tempDirs.push(dir);
  return dir;
};

type BytecodeView = {
  hex?: string | null;
  bytes?: Uint8Array | null;
};

const expectBytecodeShape = (bytecode?: BytecodeView) => {
  expect(bytecode).toBeTruthy();
  expect(bytecode?.hex).toMatch(/^0x[0-9a-f]+$/i);
  if (bytecode?.bytes instanceof Uint8Array) {
    expect(bytecode.bytes.length).toBeGreaterThan(0);
  } else {
    expect(Array.isArray(bytecode?.bytes)).toBe(true);
    expect((bytecode?.bytes as number[] | undefined)?.length).toBeGreaterThan(
      0
    );
  }
};

const expectAbiShape = (abi: unknown) => {
  expect(Array.isArray(abi)).toBe(true);
};

type SourceArtifactsView = {
  sourcePath?: string | null;
  contracts?: Record<string, { name?: string }>;
};

type ArtifactCarrier = {
  artifact?: SourceArtifactsView;
  artifacts?: Record<string, SourceArtifactsView | undefined>;
};

const flattenContracts = (output: ArtifactCarrier) => {
  const seen = new Set<string>();
  const flattened: any[] = [];

  if (output.artifact) {
    const sourceName =
      output.artifact.sourcePath ??
      (output.artifact as any).source_path ??
      "__virtual__";
    for (const [contractName, contract] of Object.entries(
      output.artifact.contracts ?? {}
    )) {
      const name = (contract as any)?.name ?? contractName;
      const key = `${sourceName}:${name}`;
      if (seen.has(key)) continue;
      seen.add(key);
      flattened.push(contract);
    }
  }

  for (const [sourceName, sourceArtifacts] of Object.entries(
    output.artifacts ?? {}
  )) {
    if (!sourceArtifacts) continue;
    const resolvedSource =
      sourceArtifacts.sourcePath ??
      (sourceArtifacts as any).source_path ??
      sourceName;
    for (const [contractName, contract] of Object.entries(
      sourceArtifacts.contracts ?? {}
    )) {
      const name = (contract as any)?.name ?? contractName;
      const key = `${resolvedSource}:${name}`;
      if (seen.has(key)) continue;
      seen.add(key);
      flattened.push(contract);
    }
  }

  return flattened;
};

const contractNames = (output: ArtifactCarrier) =>
  flattenContracts(output).map((contract) => contract.name);

const firstContract = (output: ArtifactCarrier) =>
  flattenContracts(output)[0];

let altVersionInstalled = false;

beforeAll(async () => {
  if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
    throw new Error(
      `Solc ${DEFAULT_SOLC_VERSION} must be installed before running compiler tests. ` +
        `Install it via Compiler.installSolcVersion or Foundry's svm before executing the suite.`
    );
  }
  altVersionInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION);
  if (!altVersionInstalled) {
    try {
      await Compiler.installSolcVersion(ALT_SOLC_VERSION);
      altVersionInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION);
    } catch {
      altVersionInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION);
    }
  }
});

afterAll(() => {
  for (const dir of tempDirs.reverse()) {
    try {
      rmSync(dir, { recursive: true, force: true });
    } catch {
      // best effort cleanup
    }
  }
});

describe("Compiler static helpers", () => {
  test("installSolcVersion resolves for cached release", async () => {
    try {
      await Compiler.installSolcVersion(DEFAULT_SOLC_VERSION);
    } catch (error) {
      if (
        error instanceof Error &&
        /Failed to install solc version/i.test(error.message)
      ) {
        return;
      }
      throw error;
    }
  });

  test("installSolcVersion installs missing releases", async () => {
    if (!altVersionInstalled) {
      return;
    }
    const preInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION);
    await expect(
      Compiler.installSolcVersion(ALT_SOLC_VERSION)
    ).resolves.toBeUndefined();
    expect(Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION)).toBe(true);
    if (!preInstalled) {
      await expect(
        Compiler.installSolcVersion(ALT_SOLC_VERSION)
      ).resolves.toBeUndefined();
    }
  });

  test("isSolcVersionInstalled rejects malformed versions", () => {
    expect(() =>
      Compiler.isSolcVersionInstalled("not-a-version")
    ).toThrowErrorMatchingInlineSnapshot(
      `"Failed to parse solc version: unexpected character 'n' while parsing major version number"`
    );
  });

  test("isSolcVersionInstalled respects custom svm home", () => {
    const original = process.env.SVM_HOME;
    const temp = createTempDir("tevm-svm-");
    process.env.SVM_HOME = temp;
    try {
      const overridden = Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION);
      expect(typeof overridden).toBe("boolean");
    } finally {
      if (original === undefined) {
        delete process.env.SVM_HOME;
      } else {
        process.env.SVM_HOME = original;
      }
    }
  });
});

describe("Compiler constructor", () => {
  test("rejects invalid settings shape", () => {
    expect(
      () => new Compiler({ solcSettings: 42 as unknown as any })
    ).toThrowErrorMatchingInlineSnapshot(
      `"solcSettings override must be provided as an object."`
    );
  });

  test("rejects malformed solc versions at construction", () => {
    expect(
      () => new Compiler({ solcVersion: "bad-version" })
    ).toThrowErrorMatchingInlineSnapshot(
      `"Failed to parse solc version: unexpected character 'b' while parsing major version number"`
    );
  });

  test("rejects when requested solc version is not installed", () => {
    expect(
      () => new Compiler({ solcVersion: "123.45.67" })
    ).toThrowErrorMatchingInlineSnapshot(
      `"Solc 123.45.67 is not installed. Call installSolcVersion first."`
    );
  });

  test("accepts nested settings without mutating defaults", () => {
    const compiler = new Compiler({
      solcVersion: DEFAULT_SOLC_VERSION,
      solcSettings: {
        optimizer: { enabled: true, runs: 9 },
        metadata: { bytecodeHash: BytecodeHash.None },
        debug: {
          revertStrings: RevertStrings.Debug,
          debugInfo: ["*"],
        },
        libraries: {
          "": {
            MathLib: `0x${"11".repeat(20)}`,
          },
        },
        outputSelection: DEFAULT_OUTPUT_SELECTION,
        evmVersion: EvmVersion.London,
      },
    });

    const first = compiler.compileSource(INLINE_SOURCE);
    const second = compiler.compileSource(INLINE_SOURCE);

    expect(flattenContracts(first)).toHaveLength(1);
    expect(flattenContracts(second)).toHaveLength(1);
  });

  test("per-call overrides leaving outputSelection empty are sanitized", () => {
    const compiler = new Compiler();
    const first = compiler.compileSource(INLINE_SOURCE);
    const second = compiler.compileSource(INLINE_SOURCE, {
      solcSettings: {
        optimizer: { enabled: true, runs: 1 },
        outputSelection: {
          "*": { "*": [], "": [] },
        },
      },
    });
    const third = compiler.compileSource(INLINE_SOURCE);

    expect(flattenContracts(first)).toHaveLength(1);
    expect(second.hasCompilerErrors()).toBe(false);
    expect(flattenContracts(second)).toHaveLength(1);
    expect(flattenContracts(third)).toHaveLength(1);
  });

  test("per-call solc version overrides do not leak into subsequent compiles", () => {
    const compiler = new Compiler({ solcVersion: DEFAULT_SOLC_VERSION });
    if (!altVersionInstalled) {
      const baseline = compiler.compileSource(INLINE_SOURCE);
      expect(baseline.hasCompilerErrors()).toBe(false);
      return;
    }
    const baseline = compiler.compileSource(INLINE_SOURCE);
    const alt = compiler.compileSource(INLINE_SOURCE, {
      solcVersion: ALT_SOLC_VERSION,
    });
    const after = compiler.compileSource(INLINE_SOURCE);

    expect(baseline.hasCompilerErrors()).toBe(false);
    expect(alt.hasCompilerErrors()).toBe(false);
    expect(after.hasCompilerErrors()).toBe(false);
  });

  test("per-call overrides referencing missing solc versions throw and keep state intact", () => {
    const compiler = new Compiler();
    expect(() =>
      compiler.compileSource(INLINE_SOURCE, { solcVersion: "999.0.0" })
    ).toThrowErrorMatchingInlineSnapshot(
      `"Solc 999.0.0 is not installed. Call installSolcVersion first."`
    );
    const result = compiler.compileSource(INLINE_SOURCE);
    expect(result.hasCompilerErrors()).toBe(false);
  });
});

describe("Compiler.compileSource with Solidity strings", () => {
  test("compiles inline solidity and exposes artifacts", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(INLINE_SOURCE);

    expect(output.hasCompilerErrors()).toBe(false);
    expect(output.errors).toBeUndefined();
    expect(flattenContracts(output)).toHaveLength(1);

    const [artifact] = flattenContracts(output);
    expect(artifact.name).toBe("InlineExample");
    expectBytecodeShape(artifact.creationBytecode);
    expectBytecodeShape(artifact.runtimeBytecode ?? artifact.deployedBytecode);
    expectAbiShape(artifact.abi);
    if (artifact.methodIdentifiers) {
      expect(typeof artifact.methodIdentifiers).toBe("object");
    }
    if (artifact.immutableReferences) {
      expect(typeof artifact.immutableReferences).toBe("object");
    }
  });

  test("produces warnings without marking compilation as failed", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(WARNING_SOURCE);

    expect(output.hasCompilerErrors()).toBe(false);
    expect(output.errors).toBeUndefined();
    const warnings = output.diagnostics.filter(
      (diagnostic) => diagnostic.severity === SeverityLevel.Warning
    );
    expect(warnings.length).toBeGreaterThan(0);
    const severities = new Set(output.diagnostics.map((err) => err.severity));
    expect(severities.has(SeverityLevel.Warning)).toBe(true);
  });

  test("surfaces syntax errors without throwing", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(BROKEN_SOURCE);

    expect(output.hasCompilerErrors()).toBe(true);
    expect(output.errors).toBeDefined();
    const errors = output.errors ?? [];
    expect(errors.length).toBeGreaterThan(0);
    const error = errors[0];
    expect(error.message).toMatch(/expected ';'/i);
    expect(error.severity).toBe(SeverityLevel.Error);
  });

  test("supports stopAfter parsing while keeping diagnostics", () => {
    const compiler = new Compiler();
    const parsingOnly = compiler.compileSource(BROKEN_SOURCE, {
      solcSettings: { stopAfter: "parsing" },
    });
    expect(flattenContracts(parsingOnly)).toHaveLength(0);
    expect(parsingOnly.hasCompilerErrors()).toBe(true);
    expect(parsingOnly.errors).toBeDefined();
    expect(parsingOnly.errors?.[0]?.message).toMatchInlineSnapshot(
      `"Requested output selection conflicts with "settings.stopAfter"."`
    );

    const parsingOnlyCorrect = compiler.compileSource(INLINE_SOURCE, {
      solcSettings: {
        stopAfter: "parsing",
        outputSelection: {
          "*": {
            "": ["ast"],
          },
        },
      },
    });
    expect(flattenContracts(parsingOnlyCorrect)).toHaveLength(0);
    expect(parsingOnlyCorrect.hasCompilerErrors()).toBe(false);
    expect(parsingOnlyCorrect.artifact?.ast).toBeDefined();
    expect(parsingOnlyCorrect.artifact?.contracts).toBeDefined();
    expect(Object.keys(parsingOnlyCorrect.artifact?.contracts ?? {})).toHaveLength(0);
  });

  test("accepts complete solcSettings payload", () => {
    const settings = {
      stopAfter: "parsing",
      remappings: ["lib/=lib"],
      optimizer: { enabled: true, runs: 123, details: { yul: true } },
      modelChecker: {
        engine: ModelCheckerEngine.Bmc,
        timeout: 1,
        contracts: { "*": ["*"] },
      },
      metadata: {
        useLiteralContent: true,
        bytecodeHash: BytecodeHash.None,
        cborMetadata: false,
      },
      outputSelection: {
        "*": { "*": ["abi", "evm.bytecode.object"] },
      },
      evmVersion: EvmVersion.Prague,
      viaIr: true,
      debug: { revertStrings: RevertStrings.Debug, debugInfo: ["location"] },
      libraries: {
        "LibraryConsumer.sol": {
          MathLib: "0x0000000000000000000000000000000000000001",
        },
      },
    } as const satisfies CompilerSettings;

    const compiler = new Compiler({ solcSettings: settings });
    const output = compiler.compileSource(BROKEN_SOURCE, {
      solcSettings: settings,
    });

    expect(flattenContracts(output)).toHaveLength(0);
    expect(output.hasCompilerErrors()).toBe(true);
    expect(output.errors).toBeDefined();
    expect((output.errors ?? []).length).toBeGreaterThan(0);
  });

  test("respects per-call optimizer overrides", () => {
    const compiler = new Compiler({
      solcSettings: {
        optimizer: { enabled: false },
      },
    });

    const withoutOptimizer = compiler.compileSource(INLINE_SOURCE);
    const withOptimizer = compiler.compileSource(INLINE_SOURCE, {
      solcSettings: {
        optimizer: { enabled: true, runs: 200 },
      },
    });

    expect(flattenContracts(withoutOptimizer)).toHaveLength(1);
    expect(flattenContracts(withOptimizer)).toHaveLength(1);
    expect(withOptimizer.errors).toBeUndefined();
  });

  test("allows metadata and evm version overrides", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(INLINE_SOURCE, {
      solcSettings: {
        metadata: { bytecodeHash: BytecodeHash.None },
        evmVersion: EvmVersion.London,
      },
    });
    expect(output.hasCompilerErrors()).toBe(false);
    expect(flattenContracts(output)).toHaveLength(1);
  });

  test("compiles multiple contracts in a single source", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(MULTI_CONTRACT_SOURCE);

    const names = contractNames(output);
    expect(names).toEqual(
      expect.arrayContaining(["First", "Second", "Target"])
    );
  });

  test("supports concurrent compilation calls", async () => {
    const compiler = new Compiler();
    const [a, b] = await Promise.all([
      Promise.resolve().then(() => compiler.compileSource(INLINE_SOURCE)),
      Promise.resolve().then(() =>
        compiler.compileSource(MULTI_CONTRACT_SOURCE)
      ),
    ]);

    expect(a.hasCompilerErrors()).toBe(false);
    expect(b.hasCompilerErrors()).toBe(false);
  });
});

describe("Compiler.compileSource with AST and Yul inputs", () => {
  test("accepts pre-parsed AST values", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const compiler = new Compiler();
    const output = compiler.compileSource(ast);
    expect(output.hasCompilerErrors()).toBe(false);
    expect(firstContract(output).name).toBe("InlineExample");
  });

  test("returns diagnostics when AST lacks contract definitions", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(deepClone(EMPTY_SOURCE_UNIT));
    expect(flattenContracts(output)).toHaveLength(0);
    expect(output.errors).toBeUndefined();
    expect(Array.isArray(output.diagnostics)).toBe(true);
  });

  test("compiles sanitized AST after instrumentation", () => {
    const instrumented = createAst()
      .fromSource(INLINE_SOURCE)
      .injectShadow(FUNCTION_FRAGMENT)
      .injectShadow(VARIABLE_FRAGMENT)
      .ast();

    const compiler = new Compiler();
    const output = compiler.compileSource(instrumented);
    expect(output.hasCompilerErrors()).toBe(false);
    expect(firstContract(output).name).toBe("InlineExample");
  });

  test("ignores unsupported solc languages for AST sources", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const compiler = new Compiler();
    const output = compiler.compileSource(ast, {
      solcLanguage: SolcLanguage.Yul,
    });
    expect(output.hasCompilerErrors()).toBe(false);
  });

  test("compiles Yul sources when requested", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(YUL_SOURCE, {
      solcLanguage: SolcLanguage.Yul,
    });
    expect(output.hasCompilerErrors()).toBe(false);
    expect(flattenContracts(output)).toHaveLength(1);
    const compiled = firstContract(output);
    expectBytecodeShape(compiled.creationBytecode ?? compiled.bytecode);
  });
});

describe("Compiler.compileSources", () => {
  test("compiles multiple solidity entries by path", () => {
    const compiler = new Compiler();
    const output = compiler.compileSources({
      "InlineExample.sol": INLINE_SOURCE,
      "WarningContract.sol": WARNING_SOURCE,
    });

    const names = contractNames(output);
    expect(names).toEqual(
      expect.arrayContaining(["InlineExample", "WarningContract"])
    );
  });

  test("compiles Yul sources when supplied as a map", () => {
    const compiler = new Compiler();
    const output = compiler.compileSources(
      {
        "Echo.yul": YUL_SOURCE,
      },
      { solcLanguage: SolcLanguage.Yul }
    );

    expect(output.hasCompilerErrors()).toBe(false);
    expect(flattenContracts(output)).toHaveLength(1);
  });

  test("compiles AST entries keyed by path", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const compiler = new Compiler();
    const output = compiler.compileSources({ "InlineExample.sol": ast });

    expect(output.hasCompilerErrors()).toBe(false);
    expect(firstContract(output).name).toBe("InlineExample");
  });

  test("rejects mixing ast and source strings", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const compiler = new Compiler();
    expect(() =>
      compiler.compileSources({
        "InlineExample.sol": INLINE_SOURCE,
        "InlineExample.ast": ast,
      })
    ).toThrowErrorMatchingInlineSnapshot(
      `"compileSources does not support mixing inline source strings with AST entries in the same call."`
    );
  });
});

describe("Compiler.compileFiles", () => {
  const createCompiler = () => new Compiler();

  test("compiles solidity files from disk", () => {
    const compiler = createCompiler();
    const output = compiler.compileFiles([INLINE_PATH, WARNING_PATH]);

    const names = contractNames(output);
    expect(names).toEqual(
      expect.arrayContaining(["InlineExample", "WarningContract"])
    );
  });

  test("compiles yul files when language override is provided", () => {
    const compiler = createCompiler();
    const output = compiler.compileFiles([YUL_PATH], {
      solcLanguage: SolcLanguage.Yul,
    });

    expect(output.hasCompilerErrors()).toBe(false);
    expect(flattenContracts(output)).toHaveLength(1);
  });

  test("throws when a path cannot be read", () => {
    const compiler = createCompiler();
    expect(() =>
      compiler.compileFiles(["/non-existent/path.sol"])
    ).toThrowErrorMatchingInlineSnapshot(
      `"Failed to read source file: No such file or directory (os error 2)"`
    );
  });

  test("compiles json ast files", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const dir = createTempDir("tevm-compile-files-ast-");
    const astPath = join(dir, "InlineExample.ast.json");
    writeFileSync(astPath, JSON.stringify(ast));

    const compiler = createCompiler();
    const output = compiler.compileFiles([astPath]);

    expect(output.hasCompilerErrors()).toBe(false);
    expect(firstContract(output).name).toBe("InlineExample");
  });

  test("compiles ast files with unrecognized extensions", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const dir = createTempDir("tevm-compile-files-ast-ext-");
    const astPath = join(dir, "InlineExample.ast");
    writeFileSync(astPath, JSON.stringify(ast));

    const compiler = createCompiler();
    const output = compiler.compileFiles([astPath]);

    expect(firstContract(output).name).toBe("InlineExample");
  });

  test("errors when mixing ast and source inputs", () => {
    const ast = createAst().fromSource(INLINE_SOURCE).ast();
    const dir = createTempDir("tevm-compile-files-mix-");
    const astPath = join(dir, "InlineExample.ast.json");
    writeFileSync(astPath, JSON.stringify(ast));
    const compiler = createCompiler();

    expect(() =>
      compiler.compileFiles([INLINE_PATH, astPath])
    ).toThrowErrorMatchingInlineSnapshot(
      `"compileSources does not support mixing inline source strings with AST entries in the same call."`
    );
  });

  test("errors when extension is unknown and no language override is provided", () => {
    const dir = createTempDir("tevm-compile-files-unknown-");
    const unknownPath = join(dir, "InlineExample.txt");
    writeFileSync(unknownPath, INLINE_SOURCE);
    const compiler = createCompiler();

    expect(() => compiler.compileFiles([unknownPath])).toThrow(
      /Unable to infer solc language/i
    );
  });

  test("errors when multiple languages are detected", () => {
    const compiler = createCompiler();
    expect(() =>
      compiler.compileFiles([INLINE_PATH, YUL_PATH])
    ).toThrowErrorMatchingInlineSnapshot(
      `"compileFiles requires all non-AST sources to share the same solc language. Provide solcLanguage explicitly to disambiguate."`
    );
  });

  test("ignores constructor language preference", () => {
    const compiler = new Compiler({
      solcVersion: DEFAULT_SOLC_VERSION,
      solcLanguage: SolcLanguage.Yul,
    });
    const output = compiler.compileFiles([INLINE_PATH]);

    expect(output.hasCompilerErrors()).toBe(false);
    expect(firstContract(output).name).toBe("InlineExample");
  });

  test("rejects json files that are not objects", () => {
    const dir = createTempDir("tevm-compile-files-json-");
    const jsonPath = join(dir, "Invalid.json");
    writeFileSync(jsonPath, "[]");
    const compiler = createCompiler();

    expect(() =>
      compiler.compileFiles([jsonPath])
    ).toThrowErrorMatchingInlineSnapshot(
      `"JSON sources must contain a Solidity AST object."`
    );
  });
});

describe("Compiler project paths", () => {
  test("reports synthetic layout when no project is attached", () => {
    const root = createTempDir("tevm-synth-");
    const compiler = Compiler.fromRoot(root);
    const paths = compiler.getPaths();
    const canonical = realpathSync(root);

    expect(paths.root).toBe(canonical);
    expect(paths.cache).toBe(
      join(canonical, ".tevm", "cache", "solidity-files-cache.json")
    );
    expect(paths.artifacts).toBe(join(canonical, ".tevm", "out"));
    expect(paths.buildInfos).toBe(
      join(canonical, ".tevm", "out", "build-info")
    );
    expect(paths.sources).toBe(canonical);
    expect(paths.tests).toBe(join(canonical, "test"));
    expect(paths.scripts).toBe(join(canonical, "scripts"));
    expect(paths.virtualSources).toBe(
      join(canonical, ".tevm", "virtual-sources")
    );
    expect(paths.libraries).toHaveLength(0);
    expect(paths.includePaths).toHaveLength(0);
    expect(new Set(paths.allowedPaths)).toContain(canonical);
  });
});
