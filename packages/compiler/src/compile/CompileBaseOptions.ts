import type { LogOptions } from '@tevm/logger'
import type {
	Releases,
	SolcDebugSettings,
	SolcEvmVersion,
	SolcLanguage,
	SolcMetadataSettings,
	SolcModelChecker,
	SolcOptimizer,
	SolcRemapping,
} from '@tevm/solc'
import type { FileAccessObject } from '../resolutions/FileAccessObject.js'
import type { CompilationOutputOption } from './CompilationOutputOption.js'

// All of the below options can be overridden on a per-compilation basis
export interface CompileBaseOptions<
	TLanguage extends SolcLanguage = SolcLanguage,
	TCompilationOutput extends CompilationOutputOption[] | undefined = CompilationOutputOption[] | undefined,
> {
	// Solc settings
	/**
	 * EVM version
	 *
	 * default: latest stable hardfork
	 * @see {@link SolcEvmVersion}
	 */
	hardfork?: SolcEvmVersion | undefined
	/**
	 * The compilation output selection
	 *
	 * Use '*' to select all outputs
	 *
	 * default: ['ast', 'abi', 'bytecode', 'deployedBytecode', 'storageLayout']
	 */
	compilationOutput?: TCompilationOutput | undefined
	/**
	 * Optimizer settings with optional components details
	 *
	 * default: none
	 * @see {@link SolcOptimizer}
	 */
	optimizer?: SolcOptimizer | undefined
	/**
	 * Whether to use the IR compiler
	 *
	 * default: false
	 */
	viaIR?: boolean
	/**
	 * Optional settings to inject/strip revert strings and debug assembly/Yul code
	 * @see {@link SolcDebugSettings}
	 */
	debug?: SolcDebugSettings | undefined
	/**
	 * Optional settings to specify how to handle the suffixing of the metadata
	 * @see {@link SolcMetadataSettings}
	 */
	metadata?: SolcMetadataSettings | undefined

	/**
	 * Optional experimental model checker settings
	 *
	 * @see {@link SolcModelChecker}
	 */
	modelChecker?: SolcModelChecker | undefined

	/**
	 * Remappings to apply in order to the source code of compiled contracts
	 */
	remappings?: SolcRemapping | undefined
	/**
	 * Link placeholder addresses to library addresses
	 */
	libraries?: Record<string, Record<string, string>> | undefined

	// Additional compiler settings
	/**
	 * Language of the source code
	 *
	 * Note: this can be used to set a compiler-level language, e.g. to create an AST compiler, and overriden
	 * on a per-compilation basis
	 *
	 * default: Solidity
	 * @see {@link SolcLanguage}
	 */
	language?: TLanguage | undefined
	/**
	 * Solc version
	 *
	 * If not provided, it will extract all the pragmas and use the most recent compatible version using solc-typed-ast
	 * @see {@link Releases}
	 */
	solcVersion?: keyof Releases | undefined
	/**
	 * Whether to throw on version mismatch, i.e. if the provided version is
	 * not listed as a compatible version for the provided sources
	 *
	 * default: true
	 */
	throwOnVersionMismatch?: boolean | undefined
	/**
	 * Whether to throw on a compilation error, i.e. if the compilation returns at least one error
	 *
	 * default: false
	 */
	throwOnCompilationError?: boolean | undefined
	/**
	 * Whether to cache the compilation results
	 *
	 * default: true
	 */
	cacheEnabled?: boolean | undefined
	/**
	 * Directory to cache the compilation results
	 *
	 * default: memory
	 * TODO: does it make sense to cache and default to memory?
	 */
	cacheDirectory?: string | undefined

	/**
	 * Pino logger
	 */
	loggingLevel?: LogOptions['level'] | undefined

	/**
	 * File Access Object for abstracting filesystem operations
	 *
	 * Provides a pluggable interface for reading files, enabling:
	 * - Virtual filesystems for testing
	 * - Custom resolution strategies (HTTP, database, etc.)
	 * - Instrumentation and logging
	 * - Platform-specific implementations
	 *
	 * If not provided, defaults to Node.js `fs` module.
	 *
	 * @default createDefaultFao()
	 * @example
	 * ```typescript
	 * // Virtual filesystem
	 * const virtualFs = {
	 *   'contract.sol': 'contract Foo { ... }'
	 * }
	 *
	 * const fao = {
	 *   readFile: async (path) => virtualFs[path],
	 *   readFileSync: (path) => virtualFs[path],
	 *   existsSync: (path) => path in virtualFs,
	 *   exists: async (path) => path in virtualFs
	 * }
	 *
	 * await compiler.compileFiles(['contract.sol'], { fileAccessObject: fao })
	 * ```
	 */
	fileAccessObject?: FileAccessObject | undefined

	/**
	 * Expose all internal and private functions (change their visibility to public)
	 */
	exposeInternalFunctions?: boolean | undefined
	/**
	 * Expose all internal and private variables (change their visibility to public)
	 */
	exposeInternalVariables?: boolean | undefined
}
