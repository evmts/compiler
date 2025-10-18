import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, mkdtempSync, mkdirSync, realpathSync, rmSync } from "fs";
import { basename, join } from "path";
import { tmpdir } from "os";
import { Compiler } from "../build/index.js";

const FIXTURES_DIR = join(__dirname, "fixtures");
const HARDHAT_PROJECT = join(FIXTURES_DIR, "hardhat-project");

const tempDirs: string[] = [];

const cloneHardhatProject = () => {
  const dir = mkdtempSync(join(tmpdir(), "tevm-hardhat-"));
  tempDirs.push(dir);
  const clone = join(dir, "hardhat-project");
  cpSync(HARDHAT_PROJECT, clone, { recursive: true });
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

describe("Compiler.fromHardhatRoot", () => {
  test("compileProject returns expected artifacts", () => {
    const compiler = Compiler.fromHardhatRoot(HARDHAT_PROJECT);
    const output = compiler.compileProject();

    const artifactNames = output.artifacts.map(
      (artifact: any) => artifact.contractName
    );

    expect(artifactNames).toEqual(
      expect.arrayContaining(["SimpleStorage", "Greeter", "Counter"])
    );
    expect(output.hasCompilerErrors).toBe(false);
  });

  test("compileContract returns a single matching artifact", () => {
    const compiler = Compiler.fromHardhatRoot(HARDHAT_PROJECT);
    const output = compiler.compileContract("Greeter");

    expect(output.artifacts).toHaveLength(1);
    expect(output.artifacts[0].contractName).toBe("Greeter");
    expect(output.hasCompilerErrors).toBe(false);
  });

  test("per-call overrides take precedence over inferred build info", () => {
    const compiler = Compiler.fromHardhatRoot(HARDHAT_PROJECT);
    const optimized = compiler.compileContract("SimpleStorage", {
      solcSettings: { optimizer: { enabled: true, runs: 200 } },
    });
    const unoptimized = compiler.compileContract("SimpleStorage", {
      solcSettings: { optimizer: { enabled: false } },
    });

    const optimizedBytecode = optimized.artifacts[0]?.bytecode?.hex;
    const unoptimizedBytecode = unoptimized.artifacts[0]?.bytecode?.hex;

    expect(optimizedBytecode).toBeTruthy();
    expect(unoptimizedBytecode).toBeTruthy();
    expect(unoptimizedBytecode).not.toBe(optimizedBytecode);
  });

  test("throws when the requested contract does not exist", () => {
    const compiler = Compiler.fromHardhatRoot(HARDHAT_PROJECT);
    expect(() => compiler.compileContract("DoesNotExist")).toThrow(
      /no contract found/i
    );
  });

  test("works against cloned hardhat projects", () => {
    const clone = cloneHardhatProject();
    const compiler = Compiler.fromHardhatRoot(clone);
    const output = compiler.compileProject();

    expect(output.artifacts.length).toBeGreaterThan(0);
  });

  test("exposes hardhat project paths", () => {
    const clone = cloneHardhatProject();
    for (const dir of [
      "artifacts/build-info",
      "cache",
      "contracts",
      "node_modules",
      "scripts",
      "test",
    ]) {
      mkdirSync(join(clone, dir), { recursive: true });
    }

    const compiler = Compiler.fromHardhatRoot(clone);
    const paths = compiler.getPaths();
    const canonical = realpathSync(clone);

    expect(paths.root).toBe(canonical);
    expect(paths.cache).toBe(
      join(canonical, "cache", "solidity-files-cache.json")
    );
    expect(paths.artifacts).toBe(join(canonical, "artifacts"));
    expect(paths.buildInfos).toBe(join(canonical, "artifacts", "build-info"));
    expect(paths.sources).toBe(join(canonical, "contracts"));
    expect(paths.tests).toBe(join(canonical, "test"));
    expect(basename(paths.scripts)).toBe("script");
    expect(paths.virtualSources).toBeUndefined();
    expect(new Set(paths.libraries)).toContain(join(canonical, "node_modules"));
    expect(paths.includePaths).toHaveLength(0);
    expect(new Set(paths.allowedPaths)).toContain(canonical);
  });
});
