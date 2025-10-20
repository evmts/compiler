import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, mkdtempSync, mkdirSync, realpathSync, rmSync } from "fs";
import { basename, join } from "path";
import { tmpdir } from "os";
import { Compiler } from "../build/index.js";

const FIXTURES_DIR = join(__dirname, "fixtures");
const FOUNDRY_PROJECT = join(FIXTURES_DIR, "foundry-project");

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

const contractBytecodeHex = (contract: any) =>
  contract?.creationBytecode?.hex ??
  contract?.runtimeBytecode?.hex ??
  contract?.deployedBytecode?.hex ??
  null;

const tempDirs: string[] = [];

const cloneFoundryProject = () => {
  const dir = mkdtempSync(join(tmpdir(), "tevm-foundry-"));
  tempDirs.push(dir);
  const clone = join(dir, "foundry-project");
  cpSync(FOUNDRY_PROJECT, clone, { recursive: true });
  return clone;
};

afterAll(() => {
  for (const dir of tempDirs.reverse()) {
    try {
      rmSync(dir, { recursive: true, force: true });
    } catch {
      // best effort cleanup
    }
  }
});

describe("Compiler.fromFoundryRoot", () => {
  test("compileProject returns expected artifacts", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    const output = compiler.compileProject();

    expect(contractNames(output)).toEqual(expect.arrayContaining(["Counter"]));
    expect(output.hasCompilerErrors()).toBe(false);
  });

  test("compileContract resolves a single counter artifact", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    const output = compiler.compileContract("Counter");

    expect(flattenContracts(output)).toHaveLength(1);
    expect(firstContract(output).name).toBe("Counter");
    expect(output.hasCompilerErrors()).toBe(false);
  });

  test("per-call overrides outrank project configuration", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    const optimized = compiler.compileContract("Counter", {
      solcSettings: { optimizer: { enabled: true, runs: 200 } },
    });
    const unoptimized = compiler.compileContract("Counter", {
      solcSettings: { optimizer: { enabled: false } },
    });

    const optimizedBytecode = contractBytecodeHex(firstContract(optimized));
    const unoptimizedBytecode = contractBytecodeHex(firstContract(unoptimized));

    expect(optimizedBytecode).toBeTruthy();
    expect(unoptimizedBytecode).toBeTruthy();
    expect(unoptimizedBytecode).not.toBe(optimizedBytecode);
  });

  test("constructor overrides give way to foundry config", () => {
    const root = cloneFoundryProject();
    const baseline = Compiler.fromFoundryRoot(root);
    const overridden = Compiler.fromFoundryRoot(root, {
      solcSettings: { optimizer: { runs: 1 } },
    });

    const baselineOutput = baseline.compileContract("Counter");
    const overriddenOutput = overridden.compileContract("Counter");

    const baselineContract = firstContract(baselineOutput);
    const overriddenContract = firstContract(overriddenOutput);
    const baselineBytecode = contractBytecodeHex(baselineContract);
    const overriddenBytecode = contractBytecodeHex(overriddenContract);

    expect(overriddenBytecode).toBe(baselineBytecode);
    expect(baselineContract.methodIdentifiers).toBeDefined();
  });

  test("throws when the contract is missing", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    expect(() => compiler.compileContract("MissingContract")).toThrow(
      /no contract found/i
    );
  });

  test("exposes foundry project paths", () => {
    const root = cloneFoundryProject();
    for (const dir of ["src", "test", "script", "lib", "cache"]) {
      mkdirSync(join(root, dir), { recursive: true });
    }

    const compiler = Compiler.fromFoundryRoot(root);
    const paths = compiler.getPaths();
    const canonical = realpathSync(root);

    expect(paths.root).toBe(canonical);
    expect(paths.cache).toBe(
      join(canonical, "cache", "solidity-files-cache.json")
    );
    expect(paths.artifacts).toBe(join(canonical, "out"));
    expect(paths.buildInfos).toBe(join(canonical, "out", "build-info"));
    expect(paths.sources).toBe(join(canonical, "src"));
    expect(paths.tests).toBe(join(canonical, "test"));
    expect(basename(paths.scripts)).toBe("script");
    expect(paths.virtualSources).toBeUndefined();
    expect(new Set(paths.libraries)).toContain(join(canonical, "lib"));
    expect(new Set(paths.allowedPaths)).toContain(canonical);
  });
});
