import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, mkdtempSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { Compiler, SolidityProject } from "../build/index.js";

const FIXTURES_DIR = join(__dirname, "fixtures");
const FOUNDRY_PROJECT = join(FIXTURES_DIR, "foundry-project");

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
  test("matches artifacts emitted by SolidityProject", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    const project = SolidityProject.fromDapptoolsRoot(root);

    const compilerOutput = compiler.compileProject();
    const projectOutput = project.compile();

    const compilerContracts = compilerOutput.artifacts.map(
      (artifact: any) => artifact.contractName
    );
    const projectContracts = projectOutput.artifacts.map(
      (artifact: any) => artifact.contractName
    );

    expect(compilerContracts).toEqual(
      expect.arrayContaining(projectContracts)
    );
    expect(compilerOutput.hasCompilerErrors).toBe(false);
  });

  test("compileContract resolves a single counter artifact", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    const output = compiler.compileContract("Counter");

    expect(output.artifacts).toHaveLength(1);
    expect(output.artifacts[0].contractName).toBe("Counter");
    expect(output.hasCompilerErrors).toBe(false);
  });

  test("per-call overrides outrank project configuration", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    const optimized = compiler.compileContract("Counter", {
      settings: { optimizer: { enabled: true, runs: 200 } },
    });
    const unoptimized = compiler.compileContract("Counter", {
      settings: { optimizer: { enabled: false } },
    });

    const optimizedBytecode = optimized.artifacts[0]?.bytecode;
    const unoptimizedBytecode = unoptimized.artifacts[0]?.bytecode;

    expect(optimizedBytecode).toBeTruthy();
    expect(unoptimizedBytecode).toBeTruthy();
    expect(unoptimizedBytecode).not.toBe(optimizedBytecode);
  });

  test("constructor overrides give way to foundry config", () => {
    const root = cloneFoundryProject();
    const baseline = Compiler.fromFoundryRoot(root);
    const overridden = Compiler.fromFoundryRoot(root, {
      settings: { optimizer: { runs: 1 } },
    });

    const baselineOutput = baseline.compileContract("Counter");
    const overriddenOutput = overridden.compileContract("Counter");

    const baselineBytecode = baselineOutput.artifacts[0]?.bytecode;
    const overriddenBytecode = overriddenOutput.artifacts[0]?.bytecode;

    expect(overriddenBytecode).toBe(baselineBytecode);
  });

  test("throws when the contract is missing", () => {
    const root = cloneFoundryProject();
    const compiler = Compiler.fromFoundryRoot(root);
    expect(() => compiler.compileContract("MissingContract")).toThrow(
      /no contract found/i
    );
  });
});
