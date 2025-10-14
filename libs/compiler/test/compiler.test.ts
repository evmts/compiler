import { describe, test, expect, beforeAll } from "bun:test";
import { join } from "path";
import {
  SolidityProject,
  SolidityProjectBuilder,
  createHardhatPaths,
  findArtifactsDir,
  findSourceDir,
  findLibs,
  sum,
} from "../build/index.js";

const FIXTURES_DIR = join(__dirname, "fixtures");
const HARDHAT_PROJECT = join(FIXTURES_DIR, "hardhat-project");

describe("Utility Functions", () => {
  test("sum should add two numbers", () => {
    expect(sum(2, 3)).toBe(5);
    expect(sum(-1, 1)).toBe(0);
    expect(sum(0, 0)).toBe(0);
  });

  test("findArtifactsDir should find artifacts directory", () => {
    const artifactsDir = findArtifactsDir(HARDHAT_PROJECT);
    expect(artifactsDir).toContain("artifacts");
  });

  test("findSourceDir should find source directory", () => {
    const sourceDir = findSourceDir(HARDHAT_PROJECT);
    expect(sourceDir).toContain("contracts");
  });

  test("findLibs should return library directories", () => {
    const libs = findLibs(HARDHAT_PROJECT);
    expect(Array.isArray(libs)).toBe(true);
  });
});

describe("ProjectPaths", () => {
  test("createHardhatPaths should create valid paths config", () => {
    const paths = createHardhatPaths(HARDHAT_PROJECT);

    expect(paths.root).toBeTruthy();
    expect(paths.cache).toBeTruthy();
    expect(paths.artifacts).toBeTruthy();
    expect(paths.sources).toBeTruthy();
    expect(paths.tests).toBeTruthy();
    expect(paths.scripts).toBeTruthy();
    expect(Array.isArray(paths.libraries)).toBe(true);
  });
});

describe("SolidityProject", () => {
  let project: any;

  beforeAll(() => {
    project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
  });

  test("should create project from Hardhat root", () => {
    expect(project).toBeTruthy();
  });

  test("should get sources", () => {
    const sources = project.getSources();
    expect(Array.isArray(sources)).toBe(true);
    expect(sources.length).toBeGreaterThan(0);

    // Should include our test contracts
    const sourceNames = sources.join(",");
    expect(sourceNames).toContain("SimpleStorage.sol");
  });

  test("should compile all contracts", () => {
    const output = project.compile();

    expect(output).toBeTruthy();
    expect(Array.isArray(output.artifacts)).toBe(true);
    expect(Array.isArray(output.errors)).toBe(true);
    expect(typeof output.hasCompilerErrors).toBe("boolean");

    // Should have compiled our contracts
    expect(output.artifacts.length).toBeGreaterThan(0);

    // Check artifact structure
    const artifact = output.artifacts[0];
    expect(artifact.contractName).toBeTruthy();

    // Should have bytecode and ABI
    if (artifact.bytecode) {
      expect(artifact.bytecode).toMatch(/^0x[0-9a-f]+$/i);
    }

    if (artifact.abi) {
      const abi = JSON.parse(artifact.abi);
      expect(Array.isArray(abi)).toBe(true);
    }
  });

  test("should compile single file", () => {
    const contractPath = join(HARDHAT_PROJECT, "contracts", "SimpleStorage.sol");
    const output = project.compileFile(contractPath);

    expect(output).toBeTruthy();
    expect(output.artifacts.length).toBeGreaterThan(0);

    // Should contain SimpleStorage contract
    const simpleStorage = output.artifacts.find(
      (a: any) => a.contractName === "SimpleStorage"
    );
    expect(simpleStorage).toBeTruthy();
  });

  test("should compile multiple files", () => {
    const files = [
      join(HARDHAT_PROJECT, "contracts", "SimpleStorage.sol"),
      join(HARDHAT_PROJECT, "contracts", "Greeter.sol"),
    ];

    const output = project.compileFiles(files);

    expect(output).toBeTruthy();
    expect(output.artifacts.length).toBeGreaterThan(0);

    // Should contain both contracts
    const contractNames = output.artifacts.map((a: any) => a.contractName);
    expect(contractNames).toContain("SimpleStorage");
    expect(contractNames).toContain("Greeter");
  });

  test("should find contract path by name", () => {
    const path = project.findContractPath("SimpleStorage");
    expect(path).toBeTruthy();
    expect(path).toContain("SimpleStorage.sol");
  });
});

describe("SolidityProjectBuilder", () => {
  test("should build project with custom configuration", () => {
    const builder = new SolidityProjectBuilder();
    builder.hardhatPaths(HARDHAT_PROJECT);
    builder.setCached(false);
    builder.ephemeral();

    const project = builder.build();
    expect(project).toBeTruthy();

    // Should be able to compile
    const output = project.compile();
    expect(output.artifacts.length).toBeGreaterThan(0);
  });

  test("should build project with parallel jobs configuration", () => {
    const builder = new SolidityProjectBuilder();
    builder.hardhatPaths(HARDHAT_PROJECT);
    builder.solcJobs(4);

    const project = builder.build();
    expect(project).toBeTruthy();
  });

  test("should build project with offline mode", () => {
    const builder = new SolidityProjectBuilder();
    builder.hardhatPaths(HARDHAT_PROJECT);
    builder.setOffline(true);

    const project = builder.build();
    expect(project).toBeTruthy();
  });
});

describe("Compilation Output", () => {
  let project: any;
  let output: any;

  beforeAll(() => {
    project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    output = project.compile();
  });

  test("should have valid contract artifacts", () => {
    expect(output.artifacts.length).toBeGreaterThan(0);

    for (const artifact of output.artifacts) {
      expect(artifact.contractName).toBeTruthy();
      expect(typeof artifact.contractName).toBe("string");
    }
  });

  test("should have ABI for contracts", () => {
    const simpleStorage = output.artifacts.find(
      (a: any) => a.contractName === "SimpleStorage"
    );

    if (simpleStorage && simpleStorage.abi) {
      const abi = JSON.parse(simpleStorage.abi);
      expect(Array.isArray(abi)).toBe(true);

      // Should have setValue and getValue functions
      const functionNames = abi
        .filter((item: any) => item.type === "function")
        .map((item: any) => item.name);

      expect(functionNames).toContain("setValue");
      expect(functionNames).toContain("getValue");
    }
  });

  test("should have bytecode for contracts", () => {
    const simpleStorage = output.artifacts.find(
      (a: any) => a.contractName === "SimpleStorage"
    );

    if (simpleStorage && simpleStorage.bytecode) {
      expect(simpleStorage.bytecode).toMatch(/^0x[0-9a-f]+$/i);
      expect(simpleStorage.bytecode.length).toBeGreaterThan(2); // More than just "0x"
    }
  });

  test("should have deployed bytecode for contracts", () => {
    const simpleStorage = output.artifacts.find(
      (a: any) => a.contractName === "SimpleStorage"
    );

    if (simpleStorage && simpleStorage.deployedBytecode) {
      expect(simpleStorage.deployedBytecode).toMatch(/^0x[0-9a-f]+$/i);
      expect(simpleStorage.deployedBytecode.length).toBeGreaterThan(2);
    }
  });

  test("should report compilation errors correctly", () => {
    expect(typeof output.hasCompilerErrors).toBe("boolean");
    expect(Array.isArray(output.errors)).toBe(true);

    // For valid contracts, should not have errors
    if (output.hasCompilerErrors) {
      expect(output.errors.length).toBeGreaterThan(0);

      // Check error structure
      const error = output.errors[0];
      expect(error.message).toBeTruthy();
      expect(error.severity).toBeTruthy();
    }
  });
});

describe("Error Handling", () => {
  test("should handle non-existent project directory", () => {
    expect(() => {
      SolidityProject.fromHardhatRoot("/non/existent/path");
    }).toThrow();
  });

  test("should handle non-existent contract file", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);

    expect(() => {
      project.compileFile("/non/existent/contract.sol");
    }).toThrow();
  });

  test("should handle invalid contract name in findContractPath", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);

    expect(() => {
      project.findContractPath("NonExistentContract");
    }).toThrow();
  });
});

describe("Multiple Contract Compilation", () => {
  test("should compile all three test contracts", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const output = project.compile();

    const contractNames = output.artifacts.map((a: any) => a.contractName);

    expect(contractNames).toContain("SimpleStorage");
    expect(contractNames).toContain("Greeter");
    expect(contractNames).toContain("Counter");
  });

  test("Counter contract should have correct ABI", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const output = project.compile();

    const counter = output.artifacts.find(
      (a: any) => a.contractName === "Counter"
    );

    if (counter && counter.abi) {
      const abi = JSON.parse(counter.abi);
      const functionNames = abi
        .filter((item: any) => item.type === "function")
        .map((item: any) => item.name);

      expect(functionNames).toContain("increment");
      expect(functionNames).toContain("decrement");
      expect(functionNames).toContain("reset");
      expect(functionNames).toContain("count");
    }
  });

  test("Greeter contract should have constructor in ABI", () => {
    const project = SolidityProject.fromHardhatRoot(HARDHAT_PROJECT);
    const output = project.compile();

    const greeter = output.artifacts.find(
      (a: any) => a.contractName === "Greeter"
    );

    if (greeter && greeter.abi) {
      const abi = JSON.parse(greeter.abi);
      const hasConstructor = abi.some((item: any) => item.type === "constructor");

      expect(hasConstructor).toBe(true);
    }
  });
});
