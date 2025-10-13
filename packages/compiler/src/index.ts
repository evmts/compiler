// Re-export types from solc-typed-ast
export type {
	ASTNode,
	ContractKind,
	FunctionKind,
	FunctionStateMutability,
	FunctionVisibility,
	Mutability,
	SourceUnit,
	StateVariableVisibility,
} from 'solc-typed-ast'

export {
	extractContractsFromAstNodes,
	extractContractsFromSolcOutput,
	solcSourcesToAstNodes,
} from './ast/index.js'
export { Compiler } from './Compiler.js'
export type { CreateCompilerOptions } from './CreateCompilerOptions.js'
export type { CreateCompilerResult } from './CreateCompilerResult.js'
export type {
	CompilationOutputOption,
	CompileBaseOptions,
	CompileBaseResult,
	CompileFilesResult,
	CompileSourceResult,
	CompileSourcesResult,
	CompileSourceWithShadowOptions,
} from './compile/index.js'
export {
	compileFiles,
	compileFilesWithShadow,
	compileSource,
	compileSources,
	compileSourcesWithShadow,
	compileSourceWithShadow,
} from './compile/index.js'
export type { FileAccessObject } from './resolutions/FileAccessObject.js'
