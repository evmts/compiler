import { createLogger } from '@tevm/logger'
import { extractContractsFromAstNodes } from './ast/extractContractsFromAstNodes.js'
import { extractContractsFromSolcOutput } from './ast/extractContractsFromSolcOutput.js'
import { solcSourcesToAstNodes } from './ast/solcSourcesToAstNodes.js'
import { compileSourceInternal } from './compile/compileSource.js'
import { compileSourcesWithShadowInternal } from './compile/compileSourcesWithShadow.js'
import { compileSourceWithShadowInternal } from './compile/compileSourceWithShadow.js'
import { compileContracts } from './internal/compileContracts.js'
import { defaults } from './internal/defaults.js'
import { SolcError } from './internal/errors.js'
import { getSolc } from './internal/getSolc.js'
import { mergeOptions } from './internal/mergeOptions.js'
import { readSourceFiles } from './internal/readSourceFiles.js'
import { readSourceFilesSync } from './internal/readSourceFilesSync.js'
import { validateBaseOptions } from './internal/validateBaseOptions.js'
import { createDefaultFao } from './resolutions/createDefaultFao.js'

/**
 * A stateful compiler instance with pre-configured defaults.
 *
 * The compiler instance provides a unified API for:
 * - Compiling Solidity/Yul source code and ASTs
 * - Shadow compilation for instrumentation and testing
 * - Fetching the compilation output of verified on-chain contracts
 * - Managing solc and caching
 *
 * Options passed to the constructor become defaults for all operations, but can be
 * overridden on a per-compilation basis. This allows for flexible configuration:
 * set common options once (hardfork, optimizer, output selection) while customizing
 * individual compilations as needed.
 *
 * @example
 * const compiler = new Compiler({
 *   optimizer: { enabled: true, runs: 200 },
 *   loggingLevel: 'info'
 * })
 *
 * await compiler.loadSolc('0.8.20')
 * // Use defaults
 * compiler.compileSource('contract Foo {}')
 *
 * // Override for specific compilation
 * compiler.compileSource('contract Bar {}', {
 *   optimizer: { enabled: false }
 * })
 */
export class Compiler {
	static defaultFao = createDefaultFao()
	/**
	 * @type {import('./CreateCompilerOptions.js').CreateCompilerOptions | undefined}
	 */
	options
	/**
	 * @type {import('@tevm/logger').Logger}
	 * @private
	 */
	logger
	/**
	 * @type {import('@tevm/solc').Solc | undefined}
	 * @private
	 */
	solcInstance

	/**
	 * @param {import('./CreateCompilerOptions.js').CreateCompilerOptions} [options] - Default options for all compiler operations
	 */
	constructor(options) {
		this.options = options
		this.solcInstance = options?.solc
		this.logger =
			options?.logger ?? createLogger({ name: '@tevm/compiler', level: options?.loggingLevel ?? defaults.loggingLevel })
	}

	/**
	 * @returns {import('@tevm/solc').Solc}
	 */
	requireSolcLoaded() {
		if (this.solcInstance) return this.solcInstance

		const err = new SolcError('No version of solc loaded, call loadSolc before any compilation', {
			meta: { code: 'not_loaded' },
		})
		this.logger.error(err.message)
		throw err
	}

	/**
	 * Compiles Solidity source code or a parsed AST into contracts.
	 *
	 * Accepts either:
	 * - Raw Solidity/Yul source code as a string
	 * - Parsed AST object when language is 'SolidityAST'
	 *
	 * Options merge strategy: per-call options override factory defaults.
	 *
	 * Testing options:
	 * - `exposeInternalFunctions`: Changes visibility of internal/private functions to public
	 * - `exposeInternalVariables`: Changes visibility of internal/private state variables to public
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @param {TLanguage extends 'SolidityAST' ? import('@tevm/solc').SolcAst : string} source - Source code string or AST object
	 * @param {import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> | undefined} [compileOptions] - Options for this compilation (merged with factory defaults)
	 * @returns {import('./compile/CompileSourceResult.js').CompileSourceResult<TCompilationOutput>} Compilation result with contracts, errors, and solc output
	 */
	compileSource(source, compileOptions) {
		const solc = this.requireSolcLoaded()
		const validatedOptions = validateBaseOptions(source, mergeOptions(this.options, compileOptions), this.logger)
		return compileSourceInternal(solc, source, validatedOptions, this.logger)
	}

	/**
	 * Compiles multiple sources with arbitrary paths.
	 *
	 * Unlike compileFiles, the paths do not need to correspond to filesystem paths.
	 * This is useful for:
	 * - Compiling sources from memory or network
	 * - Working with virtual file systems
	 * - Processing sources from whatsabi or other tools
	 *
	 * All sources must be the same language (Solidity, Yul, or SolidityAST).
	 *
	 * Returns a map keyed by the source paths you provide.
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @param {Record<string, TLanguage extends 'SolidityAST' ? import('@tevm/solc').SolcAst : string>} sources - Mapping of source paths to source code/AST
	 * @param {import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> | undefined} [compileOptions] - Compilation options
	 * @returns {import('./compile/CompileSourcesResult.js').CompileSourcesResult<TCompilationOutput>} Compilation results keyed by source path
	 */
	compileSources(sources, compileOptions) {
		const solc = this.requireSolcLoaded()
		const validatedOptions = validateBaseOptions(
			Object.values(sources),
			mergeOptions(this.options, compileOptions),
			this.logger,
		)
		return compileContracts(solc, sources, validatedOptions, this.logger)
	}

	/**
	 * Compiles source with shadow code injection for instrumentation or testing.
	 *
	 * Shadow compilation workflow:
	 * 1. Source is compiled to identify target contracts
	 * 2. Shadow code (Solidity/Yul) is parsed and validated
	 * 3. Shadow methods/modifiers are injected into the target contract
	 * 4. Combined source is compiled and returned
	 *
	 * When source is AST, you must specify injectIntoContractPath and injectIntoContractName
	 * if the AST contains multiple contracts. For single-contract sources, these are optional.
	 *
	 * Merge strategies for handling name conflicts:
	 * - `safe` (default): Throws compilation error if shadow method name conflicts with existing method
	 * - `replace`: Shadow method overrides existing method (source functions marked virtual, shadow as override)
	 *
	 * Note: if a function is intended to override an existing one, it should be marked as override
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @param {TLanguage extends 'SolidityAST' ? import('@tevm/solc').SolcAst : string} source - Source code or AST to augment
	 * @param {string} shadow - Shadow code to inject (Solidity or Yul)
	 * @param {(import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> & import('./compile/CompileSourceWithShadowOptions.js').CompileSourceWithShadowOptions<TLanguage>) | undefined} [compileOptions] - Compilation and injection options
	 * @returns {import('./compile/CompileSourceResult.js').CompileSourceResult<TCompilationOutput>} Compilation result with augmented contracts
	 */
	compileSourceWithShadow(source, shadow, compileOptions) {
		const solc = this.requireSolcLoaded()
		const { sourceLanguage, shadowLanguage, injectIntoContractPath, injectIntoContractName, ...baseOptions } =
			compileOptions ?? {}
		const validatedOptions = validateBaseOptions(
			source,
			{ ...mergeOptions(this.options, baseOptions), language: sourceLanguage },
			this.logger,
		)
		return compileSourceWithShadowInternal(
			solc,
			source,
			shadow,
			validatedOptions,
			{ shadowLanguage, injectIntoContractPath, injectIntoContractName },
			this.logger,
		)
	}

	/**
	 * Compiles multiple sources with shadow code injection.
	 *
	 * Similar to {@link compileSources} but with shadow code injection into a target contract.
	 * All sources are compiled together, and the shadow code is injected into the specified target contract.
	 *
	 * When compiling multiple sources, you typically MUST specify injectIntoContractPath to identify
	 * which source contains the target contract. If there are multiple contracts in that source,
	 * you must also specify injectIntoContractName.
	 *
	 * For single-source cases, these options may be inferred automatically.
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @template {string[]} TSourcePaths
	 * @param {Record<string, TLanguage extends 'SolidityAST' ? import('@tevm/solc').SolcAst : string>} sources - Mapping of source paths to source code/AST
	 * @param {string} shadow - Shadow code to inject
	 * @param {(import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> & import('./compile/CompileSourceWithShadowOptions.js').CompileSourceWithShadowOptions<TLanguage>) | undefined} [compileOptions] - Compilation and injection options
	 * @returns {import('./compile/CompileSourcesResult.js').CompileSourcesResult<TCompilationOutput, TSourcePaths>} Compilation results with injected shadow code
	 */
	compileSourcesWithShadow(sources, shadow, compileOptions) {
		const solc = this.requireSolcLoaded()
		const { sourceLanguage, shadowLanguage, injectIntoContractPath, injectIntoContractName, ...baseOptions } =
			compileOptions ?? {}
		const validatedOptions = validateBaseOptions(
			Object.values(sources),
			{ ...mergeOptions(this.options, baseOptions), language: sourceLanguage },
			this.logger,
		)
		return compileSourcesWithShadowInternal(
			solc,
			sources,
			shadow,
			validatedOptions,
			{ shadowLanguage, injectIntoContractPath, injectIntoContractName },
			this.logger,
		)
	}

	/**
	 * Compiles multiple source files from the filesystem.
	 *
	 * All files in a single compilation must use the same language/extension:
	 * - .sol files (Solidity)
	 * - .yul files (Yul)
	 * - .json files (SolidityAST)
	 *
	 * Returns a map keyed by original file paths, allowing you to correlate
	 * compilation results back to source files.
	 *
	 * Testing options:
	 * - `exposeInternalFunctions`: Changes visibility of internal/private functions to public
	 * - `exposeInternalVariables`: Changes visibility of internal/private state variables to public
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @template {string[]} TSourcePaths
	 * @param {TSourcePaths} filePaths - Array of file paths to compile
	 * @param {import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> | undefined} [compileOptions] - Compilation options
	 * @returns {Promise<import('./compile/CompileFilesResult.js').CompileFilesResult<TCompilationOutput, TSourcePaths>>} Promise resolving to compilation results keyed by file path
	 */
	async compileFiles(filePaths, compileOptions) {
		const solc = this.requireSolcLoaded()
		const mergedOptions = mergeOptions(this.options, compileOptions)
		const sources = await readSourceFiles(
			mergedOptions.fileAccessObject ?? Compiler.defaultFao,
			filePaths,
			mergedOptions.language,
			this.logger,
		)
		const validatedOptions = validateBaseOptions(Object.values(sources), mergedOptions, this.logger)
		return /** @type {any} */ (compileContracts(solc, /** @type {any} */ (sources), validatedOptions, this.logger))
	}

	/**
	 * Compiles multiple source files from the filesystem (sync).
	 *
	 * All files in a single compilation must use the same language/extension:
	 * - .sol files (Solidity)
	 * - .yul files (Yul)
	 * - .json files (SolidityAST)
	 *
	 * Returns a map keyed by original file paths, allowing you to correlate
	 * compilation results back to source files.
	 *
	 * Testing options:
	 * - `exposeInternalFunctions`: Changes visibility of internal/private functions to public
	 * - `exposeInternalVariables`: Changes visibility of internal/private state variables to public
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @template {string[]} TSourcePaths
	 * @param {TSourcePaths} filePaths - Array of file paths to compile
	 * @param {import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> | undefined} [compileOptions] - Compilation options
	 * @returns {import('./compile/CompileFilesResult.js').CompileFilesResult<TCompilationOutput, TSourcePaths>} Compilation results keyed by file path
	 */
	compileFilesSync(filePaths, compileOptions) {
		const solc = this.requireSolcLoaded()
		const mergedOptions = mergeOptions(this.options, compileOptions)
		const sources = readSourceFilesSync(
			mergedOptions.fileAccessObject ?? Compiler.defaultFao,
			filePaths,
			mergedOptions.language,
			this.logger,
		)
		const validatedOptions = validateBaseOptions(Object.values(sources), mergedOptions, this.logger)
		return /** @type {any} */ (compileContracts(solc, /** @type {any} */ (sources), validatedOptions, this.logger))
	}

	/**
	 * Compiles multiple source files from the filesystem with shadow code injection.
	 *
	 * Similar to {@link compileFiles} but with shadow code injection into a target contract.
	 * You MUST specify injectIntoContractPath to identify which file contains the target contract.
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @template {string[]} TSourcePaths
	 * @param {TSourcePaths} filePaths - Array of file paths to compile
	 * @param {string} shadow - Shadow code to inject
	 * @param {(import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> & import('./compile/CompileSourceWithShadowOptions.js').CompileSourceWithShadowOptions<TLanguage>) | undefined} [compileOptions] - Compilation and injection options
	 * @returns {Promise<import('./compile/CompileFilesResult.js').CompileFilesResult<TCompilationOutput, TSourcePaths>>} Promise resolving to compilation results with injected shadow code
	 */
	async compileFilesWithShadow(filePaths, shadow, compileOptions) {
		const solc = this.requireSolcLoaded()
		const { sourceLanguage, shadowLanguage, injectIntoContractPath, injectIntoContractName, ...baseOptions } =
			compileOptions ?? {}
		const mergedOptions = mergeOptions(this.options, baseOptions)
		const sources = await readSourceFiles(
			mergedOptions.fileAccessObject ?? Compiler.defaultFao,
			filePaths,
			sourceLanguage,
			this.logger,
		)
		const validatedOptions = validateBaseOptions(
			/** @type {any} */ (Object.values(sources)),
			{ ...mergedOptions, language: sourceLanguage },
			this.logger,
		)
		return /** @type {any} */ (
			compileSourcesWithShadowInternal(
				solc,
				/** @type {any} */ (sources),
				shadow,
				validatedOptions,
				{ shadowLanguage, injectIntoContractPath, injectIntoContractName },
				this.logger,
			)
		)
	}

	/**
	 * Compiles multiple source files from the filesystem with shadow code injection (sync).
	 *
	 * Similar to {@link compileFiles} but with shadow code injection into a target contract.
	 * You MUST specify injectIntoContractPath to identify which file contains the target contract.
	 *
	 * @template {import('@tevm/solc').SolcLanguage} TLanguage
	 * @template {import('./compile/CompilationOutputOption.js').CompilationOutputOption[]} TCompilationOutput
	 * @template {string[]} TSourcePaths
	 * @param {TSourcePaths} filePaths - Array of file paths to compile
	 * @param {string} shadow - Shadow code to inject
	 * @param {(import('./compile/CompileBaseOptions.js').CompileBaseOptions<TLanguage, TCompilationOutput> & import('./compile/CompileSourceWithShadowOptions.js').CompileSourceWithShadowOptions<TLanguage>) | undefined} [compileOptions] - Compilation and injection options
	 * @returns {import('./compile/CompileFilesResult.js').CompileFilesResult<TCompilationOutput, TSourcePaths>} Compilation results with injected shadow code
	 */
	compileFilesWithShadowSync(filePaths, shadow, compileOptions) {
		const solc = this.requireSolcLoaded()
		const { sourceLanguage, shadowLanguage, injectIntoContractPath, injectIntoContractName, ...baseOptions } =
			compileOptions ?? {}
		const mergedOptions = mergeOptions(this.options, baseOptions)
		const sources = readSourceFilesSync(
			mergedOptions.fileAccessObject ?? Compiler.defaultFao,
			filePaths,
			sourceLanguage,
			this.logger,
		)
		const validatedOptions = validateBaseOptions(
			/** @type {any} */ (Object.values(sources)),
			{ ...mergedOptions, language: sourceLanguage },
			this.logger,
		)
		return /** @type {any} */ (
			compileSourcesWithShadowInternal(
				solc,
				/** @type {any} */ (sources),
				shadow,
				validatedOptions,
				{ shadowLanguage, injectIntoContractPath, injectIntoContractName },
				this.logger,
			)
		)
	}

	/**
	 * Extracts Solidity source code from solc compiler output.
	 *
	 * Uses solc-typed-ast's ASTWriter to regenerate source from the compiled AST.
	 * This enables AST manipulation workflows:
	 * 1. Compile source to get AST
	 * 2. Modify AST programmatically
	 * 3. Compile the instrumented AST
	 *
	 * Returns a map of source paths to regenerated Solidity code.
	 *
	 * @param {import('@tevm/solc').SolcOutput} solcOutput - Complete solc compilation output
	 * @param {import('./compile/CompileBaseOptions.js').CompileBaseOptions | undefined} [compileOptions] - Options controlling source generation
	 * @returns {{ [sourcePath: string]: string }} Map of file paths to regenerated source code
	 * @example
	 * import { ASTReader } from 'solc-typed-ast'
	 *
	 * // 1. Compile to get AST
	 * const result = await compiler.compileSource('contract Foo { uint x; }', { language: 'Solidity', compilationOutput: ['ast'] })
	 *
	 * // 2. Parse and manipulate AST
	 * const reader = new ASTReader()
	 * const sourceUnits = reader.read(result.solcOutput)
	 * const someContract = sourceUnits[0].vContracts.find(contract => contract.name === 'SomeContract')
	 * // ... manipulate the SourceUnit
	 *
	 * // 3. Compile the manipulated AST directly
	 * const instrumentedResult = await compiler.compileSource(sourceUnits[0], { language: 'SolidityAST', compilationOutput: ['bytecode'] })
	 */
	extractContractsFromSolcOutput(solcOutput, compileOptions) {
		return extractContractsFromSolcOutput(solcOutput, mergeOptions(this.options, compileOptions))
	}

	/**
	 * Convert SourceUnit AST nodes to Solidity source code
	 *
	 * @template {boolean} TWithSourceMap
	 * @param {import('solc-typed-ast').SourceUnit[]} sourceUnits - Array of source units (from solcSourcesToAstNodes)
	 * @param {(import('./compile/CompileBaseOptions.js').CompileBaseOptions & { withSourceMap?: TWithSourceMap }) | undefined} [compileOptions] - Configuration options
	 * @returns {{
	 *   sources: { [sourcePath: string]: string }
	 *   sourceMaps: TWithSourceMap extends true ? { [sourcePath: string]: Map<import('solc-typed-ast').ASTNode, [number, number]> } : undefined
	 * }} Object containing sources mapping and optional source maps
	 * @example
	 * import { extractContractsFromAstNodes } from './extractContractsFromAstNodes.js'
	 * import { solcSourcesToAstNodes } from '../internal/solcSourcesToAstNodes.js'
	 * import { createLogger } from '@tevm/logger'
	 *
	 * const logger = createLogger({ name: '@tevm/compiler' })
	 * const sourceUnits = solcSourcesToAstNodes(solcOutput.sources, logger)
	 *
	 * // Manipulate the AST source units...
	 *
	 * // Without source maps
	 * const { sources } = extractContractsFromAstNodes(sourceUnits, {
	 *   solcVersion: '0.8.20'
	 * })
	 *
	 * // With source maps
	 * const { sources, sourceMaps } = extractContractsFromAstNodes(sourceUnits, {
	 *   solcVersion: '0.8.20',
	 *   withSourceMap: true
	 * })
	 */
	extractContractsFromAstNodes(sourceUnits, compileOptions) {
		return extractContractsFromAstNodes(sourceUnits, {
			...compileOptions,
			...mergeOptions(this.options, compileOptions),
			language: 'SolidityAST',
		})
	}

	/**
	 * Parse sources object from SolcOutput['sources'] into typed AST SourceUnit nodes
	 *
	 * @param {{ [sourceFile: string]: import('@tevm/solc').SolcSourceEntry }} sources - The sources object from SolcOutput.sources
	 * @returns {import('solc-typed-ast').SourceUnit[]} Array of all source units from compilation
	 * @example
	 * import { solcSourcesToAstNodes } from './solcSourcesToAstNodes.js'
	 *
	 * const sources = solcOutput.sources
	 * const sourceUnits = solcSourcesToAstNodes(sources)
	 * // Returns array of all SourceUnits with cross-references intact
	 */
	solcSourcesToAstNodes(sources) {
		return solcSourcesToAstNodes(sources, this.logger)
	}

	/**
	 * Fetches verified source code for a deployed contract from block explorers.
	 *
	 * Uses whatsabi to:
	 * 1. Query block explorers (Blockscout, Etherscan, Sourcify) for verified source
	 * 2. Retrieve the source code and solc compilation output
	 * 3. Return in the same format as compileSource for consistency
	 *
	 * Requires API keys for block explorers to be configured in options.
	 *
	 * @param {unknown} _address - On-chain contract address
	 * @param {unknown} _whatsabiOptions - Chain config and API keys
	 * @returns {Promise<void>}
	 * TODO: get types from @tevm/whatsabi
	 */
	async fetchVerifiedSource(_address, _whatsabiOptions) {
		// TODO: implement whatsabi integration
	}

	/**
	 * Loads a specific solc compiler version into the cache (or latest if no version is provided).
	 *
	 * Solc binaries are only downloaded when using this function, which should be done
	 * before any compilation. Only `extractContractsFromSolcOutput` and `extractContractsFromAst`
	 * can be used without solc.
	 *
	 * @param {keyof import('@tevm/solc').Releases} [version] - Solc version to load (e.g., '0.8.17'). Defaults to latest if not provided.
	 * @returns {Promise<void>}
	 */
	async loadSolc(version) {
		this.solcInstance = await getSolc(version ?? defaults.solcVersion, this.logger)
	}

	/**
	 * Clears the compiled contracts cache.
	 *
	 * Removes all cached solc binaries from disk. Use when:
	 * - Updating to newer compiler versions
	 * - Freeing disk space
	 * - Troubleshooting corrupted downloads
	 * - Running tests that need clean state
	 *
	 * Compilers will be re-downloaded on next use.
	 *
	 * @returns {Promise<void>}
	 */
	async clearCache() {
		// TODO: implement cache clearing
	}
}
