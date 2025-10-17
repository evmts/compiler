import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, mkdtempSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { Compiler, SolidityProject } from "../build/index.js";

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
  test("produces the same artifacts as SolidityProject", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const projectOutput = project.compile();

    const compiler = Compiler.fromHardhatRoot(HARDHAT_PROJECT);
    const output = compiler.compileProject();

    const expectedNames = projectOutput.artifacts.map(
      (artifact: any) => artifact.contractName
    );
    const artifactNames = output.artifacts.map(
      (artifact: any) => artifact.contractName
    );

    expect(artifactNames).toEqual(expect.arrayContaining(expectedNames));
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
      settings: { optimizer: { enabled: true, runs: 200 } },
    });
    const unoptimized = compiler.compileContract("SimpleStorage", {
      settings: { optimizer: { enabled: false } },
    });

    const optimizedBytecode = optimized.artifacts[0]?.bytecode;
    const unoptimizedBytecode = unoptimized.artifacts[0]?.bytecode;

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
});
