import { beforeAll, describe, expect, test } from "bun:test";
import { join } from "path";
import {
  BytecodeHash,
  Compiler,
  EvmVersion,
  RevertStrings,
  SolidityProject,
  SolidityProjectBuilder,
  createHardhatPaths,
  findArtifactsDir,
  findLibs,
  findSourceDir,
} from "../build/index.js";
import type { OutputSelection } from "../src/types/solc-types.js";

const DEFAULT_SOLC_VERSION = "0.8.30";
const FIXTURES_DIR = join(__dirname, "fixtures");
const HARDHAT_PROJECT = join(FIXTURES_DIR, "hardhat-project");
const SIMPLE_STORAGE_PATH = join(HARDHAT_PROJECT, "contracts", "SimpleStorage.sol");
const GREETER_PATH = join(HARDHAT_PROJECT, "contracts", "Greeter.sol");
const COUNTER_PATH = join(HARDHAT_PROJECT, "contracts", "Counter.sol");

const INLINE_SOURCE = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract InlineExample {
  uint256 private stored;

  function set(uint256 newValue) external {
    stored = newValue;
  }

  function get() external view returns (uint256) {
    return stored;
  }
}
`;

const BROKEN_SOURCE = `pragma solidity ^0.8.0;

contract Broken {
  function fail() public {
    uint256 value = 1
  }
}
`;

const DEFAULT_OUTPUT_SELECTION = {
  "*": {
    "*": ["abi", "evm.bytecode", "evm.deployedBytecode", "evm.methodIdentifiers"],
    "": ["ast"],
  },
} as const satisfies OutputSelection;

beforeAll(() => {
  if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
    throw new Error(
      `Solc ${DEFAULT_SOLC_VERSION} must be installed before running compiler tests. ` +
        `Install it via Compiler.installSolcVersion or Foundry's svm before executing the suite.`,
    );
  }
});

describe("Compiler API", () => {
  test("rejects invalid settings shape at construction time", () => {
    expect(() => new Compiler({ settings: 42 as unknown as any })).toThrow(/settings override must be provided/i);
  });

  test("rejects malformed semantic versions", () => {
    expect(() => new Compiler({ solcVersion: "not-a-version" })).toThrow(/Failed to parse solc version/i);
  });

  test("rejects when requested solc version is not installed", () => {
    expect(
      () =>
        new Compiler({
          solcVersion: "999.0.0",
        }),
    ).toThrow(/Solc 999\.0\.0 is not installed/i);
  });

  test("resolves install requests immediately for cached releases", async () => {
    await expect(Compiler.installSolcVersion(DEFAULT_SOLC_VERSION)).resolves.toBeUndefined();
  });

  test("reports version availability accurately", () => {
    expect(Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)).toBe(true);
    expect(Compiler.isSolcVersionInstalled("123.45.67")).toBe(false);
  });

  test("compiles inline solidity and exposes artifacts", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(INLINE_SOURCE);

    expect(output.hasCompilerErrors).toBe(false);
    expect(output.errors).toHaveLength(0);
    expect(output.artifacts).toHaveLength(1);

    const [artifact] = output.artifacts;
    expect(artifact.contractName).toBe("InlineExample");
    expect(artifact.bytecode).toMatch(/^0x[0-9a-f]+$/i);
    expect(artifact.deployedBytecode).toMatch(/^0x[0-9a-f]+$/i);

    if (artifact.abi) {
      const abi = JSON.parse(artifact.abi);
      const fnNames = abi.filter((item: any) => item.type === "function").map((item: any) => item.name);
      expect(fnNames).toEqual(expect.arrayContaining(["set", "get"]));
    }
  });

  test("supports deeply nested typed compiler settings", () => {
    const compiler = new Compiler({
      solcVersion: DEFAULT_SOLC_VERSION,
      settings: {
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

    const typedSource = INLINE_SOURCE.replace(/\bstored\b/g, "storedValue");

    const output = compiler.compileSource(typedSource);

    expect(output.hasCompilerErrors).toBe(false);
    expect(output.errors).toHaveLength(0);
  });

  test("accepts per-call overrides without mutating constructor configuration", () => {
    const compiler = new Compiler({
      solcVersion: DEFAULT_SOLC_VERSION,
    });
    const first = compiler.compileSource(INLINE_SOURCE);
    const second = compiler.compileSource(INLINE_SOURCE, {
      settings: {
        optimizer: { enabled: true, runs: 1 },
        outputSelection: {
          "*": {
            "*": [],
            "": [],
          },
        },
      },
    });

    const third = compiler.compileSource(INLINE_SOURCE);

    expect(first.hasCompilerErrors).toBe(false);
    expect(second.hasCompilerErrors).toBe(false);
    expect(third.hasCompilerErrors).toBe(false);
    expect(first.artifacts).toHaveLength(1);
    expect(second.artifacts).toHaveLength(0);
    expect(third.artifacts).toHaveLength(1);
    expect(first.artifacts[0].contractName).toBe("InlineExample");
    expect(third.artifacts[0].contractName).toBe("InlineExample");
    expect(third.artifacts[0].abi).toBe(first.artifacts[0].abi);
  });

  test("allows per-call version overrides when binaries are installed", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(INLINE_SOURCE, { solcVersion: DEFAULT_SOLC_VERSION });
    expect(output.hasCompilerErrors).toBe(false);
  });

  test("throws when per-call overrides reference missing solc versions", () => {
    const compiler = new Compiler();
    expect(() =>
      compiler.compileSource(INLINE_SOURCE, { solcVersion: "123.45.67" }),
    ).toThrow(/not installed/i);
  });

  test("supports compiling without explicit filenames", () => {
    const compiler = new Compiler();
    const output = compiler.compileSource(INLINE_SOURCE);

    expect(output.hasCompilerErrors).toBe(false);
    expect(output.artifacts[0].contractName).toBe("InlineExample");
  });

  test("surfaces compilation diagnostics without throwing", () => {
    const compiler = new Compiler();
    const diagnostics = compiler.compileSource(BROKEN_SOURCE);

    expect(diagnostics.hasCompilerErrors).toBe(true);
    expect(diagnostics.errors.length).toBeGreaterThan(0);
    const error = diagnostics.errors[0];
    expect(error.message).toMatch(/expected ';'/i);
    expect(error.severity).toBe("Error");
  });
});

describe("Path helpers", () => {
  test("createHardhatPaths mirrors the expected project layout", () => {
    const paths = createHardhatPaths(HARDHAT_PROJECT);

    expect(paths.root).toContain("hardhat-project");
    expect(paths.sources).toContain("contracts");
    expect(paths.artifacts).toContain("artifacts");
    expect(Array.isArray(paths.libraries)).toBe(true);
  });

  test("findArtifactsDir, findSourceDir and findLibs return sensible values", () => {
    expect(findArtifactsDir(HARDHAT_PROJECT)).toContain("artifacts");
    expect(findSourceDir(HARDHAT_PROJECT)).toContain("contracts");

    const libs = findLibs(HARDHAT_PROJECT);
    expect(Array.isArray(libs)).toBe(true);
    expect(libs.length).toBeGreaterThan(0);
  });
});

describe("SolidityProject facade", () => {
  test("compiles full hardhat project and exposes artifacts", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const output = project.compile();

    const artifactNames = output.artifacts.map((artifact: any) => artifact.contractName);
    expect(artifactNames).toEqual(expect.arrayContaining(["SimpleStorage", "Greeter", "Counter"]));
    expect(output.hasCompilerErrors).toBe(false);
  });

  test("compileFile returns only the requested artifacts", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const output = project.compileFile(SIMPLE_STORAGE_PATH);

    expect(output.artifacts).toHaveLength(1);
    expect(output.artifacts[0].contractName).toBe("SimpleStorage");
  });

  test("compileFiles merges results from multiple sources", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const output = project.compileFiles([SIMPLE_STORAGE_PATH, GREETER_PATH]);
    const names = output.artifacts.map((artifact: any) => artifact.contractName);

    expect(names).toEqual(expect.arrayContaining(["SimpleStorage", "Greeter"]));
  });

  test("findContractPath resolves the on-disk file", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const path = project.findContractPath("Counter");

    expect(path.endsWith("Counter.sol")).toBe(true);
  });

  test("getSources lists all Solidity entries", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const sources = project.getSources();

    expect(Array.isArray(sources)).toBe(true);
    expect(sources.some((source) => source.endsWith("SimpleStorage.sol"))).toBe(true);
    expect(sources.some((source) => source.endsWith("Greeter.sol"))).toBe(true);
  });
});

describe("SolidityProjectBuilder", () => {
  const hardhatRoot = HARDHAT_PROJECT;

  test("method chaining preserves the builder instance", () => {
    const builder = new SolidityProjectBuilder();
    expect(builder.ephemeral()).toBe(builder);
    expect(builder.setCached(false)).toBe(builder);
    expect(builder.offline()).toBe(builder);
    expect(builder.setOffline(true)).toBe(builder);
    expect(builder.noArtifacts()).toBe(builder);
    expect(builder.setNoArtifacts(false)).toBe(builder);
    expect(builder.singleSolcJobs()).toBe(builder);
    expect(builder.solcJobs(1)).toBe(builder);
  });

  test("builds projects after applying configuration toggles", () => {
    const builder = new SolidityProjectBuilder();
    builder.hardhatPaths(hardhatRoot);
    builder
      .ephemeral()
      .setCached(false)
      .offline()
      .setOffline(false)
      .noArtifacts()
      .setNoArtifacts(true)
      .singleSolcJobs()
      .solcJobs(1);

    const project = builder.build();
    const output = project.compileFile(SIMPLE_STORAGE_PATH);

    expect(output.artifacts).toHaveLength(1);
    expect(output.artifacts[0].contractName).toBe("SimpleStorage");
  });
});
