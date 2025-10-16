/**
 * Mirrors `foundry_compilers::artifacts::output_selection` to provide a fully typed
 * representation of the standard‑JSON `outputSelection` structure.
 *
 * The map is keyed by source file (the special `"*"` entry applies to every file)
 * and points to another map keyed by contract name (where `""` refers to file-level
 * outputs and `"*"` applies to every contract within the file). Each contract entry
 * lists the concrete outputs that should be emitted by solc. Wildcards (`"*"` at the
 * contract level or as an output) retain the same semantics as the Rust implementation.
 *
 * This definition intentionally includes every synonym that Foundry’s parser accepts
 * (e.g. `ir-optimized`, `asm`, `runtime-code`, …) so TypeScript callers can rely on
 * the same permissive surface as the underlying compiler.
 */

// @ts-ignore
const fileLevelOption = '' as const
export type OutputSelection = {
  [fileSelector: string]: {
      [fileLevelOption]?: ReadonlyArray<'ast'>;
  } & {
    [contractName: Exclude<string, typeof fileLevelOption>]: ReadonlyArray<BaseContractOutput | EvmOutput | EwasmOutput>;
  };
};

type BaseContractOutput =
  // TODO: 'ast' should only be for file level but not sure how to type correctly
  | "ast"
  | "abi"
  | "devdoc"
  | "userdoc"
  | "metadata"
  | "ir"
  | "irOptimized"
  | "ir-optimized"
  | "iroptimized"
  | "storageLayout"
  | "storage-layout"
  | "storagelayout";
type EvmOutput =
  | "evm"
  | "evm.assembly"
  | "asm"
  | "evm.legacyAssembly"
  | "evm.methodIdentifiers"
  | "evm.methodidentifiers"
  | "methodidentifiers"
  | "evm.gasEstimates"
  | "evm.gasestimates"
  | "gas"
  | BytecodeOutput
  | DeployedBytecodeOutput;
type BytecodeOutput =
  | "evm.bytecode"
  | "evm.bytecode.functionDebugData"
  | "evm.bytecode.object"
  | "code"
  | "bin"
  | "evm.bytecode.opcodes"
  | "evm.bytecode.sourceMap"
  | "evm.bytecode.linkReferences"
  | "evm.bytecode.generatedSources";
type DeployedBytecodeOutput =
  | "evm.deployedBytecode"
  | "evm.deployedBytecode.functionDebugData"
  | "evm.deployedBytecode.object"
  | "deployed-code"
  | "deployed-bin"
  | "runtime-code"
  | "runtime-bin"
  | "evm.deployedBytecode.opcodes"
  | "evm.deployedBytecode.sourceMap"
  | "evm.deployedBytecode.linkReferences"
  | "evm.deployedBytecode.generatedSources"
  | "evm.deployedBytecode.immutableReferences";
type EwasmOutput =
  | "ewasm"
  | "ewasm.wast"
  | "ewasm.wasm";
