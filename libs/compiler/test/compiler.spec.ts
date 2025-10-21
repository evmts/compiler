import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
	Ast,
	BytecodeHash,
	CompileOutput,
	Compiler,
	CompilerLanguage,
	CompilerSettings,
	ContractBytecode,
	EvmVersion,
	ModelCheckerEngine,
	RevertStrings,
	SeverityLevel,
} from '../build/index.js'
import type { OutputSelection } from '../build/solc-settings.js'

const DEFAULT_SOLC_VERSION = '0.8.30'
const ALT_SOLC_VERSION = '0.8.29'
const FIXTURES_DIR = join(__dirname, 'fixtures')
const CONTRACTS_DIR = join(FIXTURES_DIR, 'contracts')
const FRAGMENTS_DIR = join(FIXTURES_DIR, 'fragments')
const AST_DIR = join(FIXTURES_DIR, 'ast')
const YUL_DIR = join(FIXTURES_DIR, 'yul')
const VYPER_DIR = join(FIXTURES_DIR, 'vyper')
const HARDHAT_PROJECT = join(FIXTURES_DIR, 'hardhat-project')
const _SIMPLE_STORAGE_PATH = join(HARDHAT_PROJECT, 'contracts', 'SimpleStorage.sol')
const INLINE_PATH = join(CONTRACTS_DIR, 'InlineExample.sol')
const BROKEN_PATH = join(CONTRACTS_DIR, 'BrokenExample.sol')
const MULTI_CONTRACT_PATH = join(CONTRACTS_DIR, 'MultiContract.sol')
const WARNING_PATH = join(CONTRACTS_DIR, 'WarningContract.sol')
const LIBRARY_PATH = join(CONTRACTS_DIR, 'MathLib.sol')
const LIBRARY_CONSUMER_PATH = join(CONTRACTS_DIR, 'LibraryConsumer.sol')
const INLINE_SOURCE = readFileSync(INLINE_PATH, 'utf8')
const BROKEN_SOURCE = readFileSync(BROKEN_PATH, 'utf8')
const MULTI_CONTRACT_SOURCE = readFileSync(MULTI_CONTRACT_PATH, 'utf8')
const WARNING_SOURCE = readFileSync(WARNING_PATH, 'utf8')
const _LIBRARY_SOURCE = readFileSync(LIBRARY_PATH, 'utf8')
const _LIBRARY_CONSUMER_SOURCE = readFileSync(LIBRARY_CONSUMER_PATH, 'utf8')
const FUNCTION_FRAGMENT = readFileSync(join(FRAGMENTS_DIR, 'function_fragment.sol'), 'utf8')
const VARIABLE_FRAGMENT = readFileSync(join(FRAGMENTS_DIR, 'variable_fragment.sol'), 'utf8')
const EMPTY_SOURCE_UNIT = JSON.parse(readFileSync(join(AST_DIR, 'empty_source_unit.json'), 'utf8'))
const _FRAGMENT_WITHOUT_TARGET = JSON.parse(readFileSync(join(AST_DIR, 'fragment_without_contract.json'), 'utf8'))
const YUL_PATH = join(YUL_DIR, 'Echo.yul')
const YUL_SOURCE = readFileSync(YUL_PATH, 'utf8')
const VYPER_COUNTER_PATH = join(VYPER_DIR, 'Counter.vy')
const VYPER_COUNTER_SOURCE = readFileSync(VYPER_COUNTER_PATH, 'utf8')


const DEFAULT_OUTPUT_SELECTION = {
	'*': {
		'*': ['abi', 'evm.bytecode', 'evm.deployedBytecode', 'evm.methodIdentifiers'],
		'': ['ast'],
	},
} as const satisfies OutputSelection

const tempDirs: string[] = []

const deepClone = <T>(value: T): T => JSON.parse(JSON.stringify(value))

const createTempDir = (prefix: string) => {
	const dir = mkdtempSync(join(tmpdir(), prefix))
	tempDirs.push(dir)
	return dir
}

const COMPILER_MODULE_PATH = join(__dirname, '../build/index.js')

const listJsonFiles = (directory: string): string[] => {
	if (!existsSync(directory)) return []
	const entries = readdirSync(directory, { withFileTypes: true })
	const files: string[] = []
	for (const entry of entries) {
		const resolved = join(directory, entry.name)
		if (entry.isDirectory()) {
			files.push(...listJsonFiles(resolved))
		} else if (entry.isFile() && entry.name.endsWith('.json')) {
			files.push(resolved)
		}
	}
	return files
}

const expectBytecodeShape = (bytecode?: ContractBytecode | null) => {
	expect(bytecode).toBeTruthy()
	expect(bytecode?.hex).toMatch(/^0x[0-9a-f]+$/i)
	if (bytecode?.bytes instanceof Uint8Array) {
		expect(bytecode.bytes.length).toBeGreaterThan(0)
	} else {
		expect(Array.isArray(bytecode?.bytes)).toBe(true)
		expect((bytecode?.bytes as number[] | undefined)?.length).toBeGreaterThan(0)
	}
}

const expectAbiShape = (abi: unknown) => {
	expect(Array.isArray(abi)).toBe(true)
}

const flattenContracts = <THasErrors extends boolean>(output: CompileOutput<THasErrors>) => {
	if (output.hasCompilerErrors()) {
		throw new Error(
			`Expected compilation without errors but received error: ${JSON.stringify(output.errors, null, 2)}`,
		)
	}
	const json = output.toJson()
	console.log('DEBUG json', json)
	console.log('DEBUG artifact', output.artifact)
	const primary = Object.values(json.artifact?.contracts ?? {})
	const extra = Object.values(json.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	return [...primary, ...extra]
}

let altVersionInstalled = false

beforeAll(async () => {
	if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
		throw new Error(
			`Solc ${DEFAULT_SOLC_VERSION} must be installed before running compiler tests. ` +
				`Install it via Compiler.installSolcVersion or Foundry's svm before executing the suite.`,
		)
	}
	altVersionInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION)
	if (!altVersionInstalled) {
		try {
			await Compiler.installSolcVersion(ALT_SOLC_VERSION)
			altVersionInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION)
		} catch {
			altVersionInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION)
		}
	}
})

afterAll(() => {
	for (const dir of tempDirs.reverse()) {
		try {
			rmSync(dir, { recursive: true, force: true })
		} catch {
			// best effort cleanup
		}
	}
})

describe('Compiler static helpers', () => {
	test('installSolcVersion resolves for cached release', async () => {
		try {
			await Compiler.installSolcVersion(DEFAULT_SOLC_VERSION)
		} catch (error) {
			if (error instanceof Error && /Failed to install solc version/i.test(error.message)) {
				return
			}
			throw error
		}
	})

	test('installSolcVersion installs missing releases', async () => {
		if (!altVersionInstalled) {
			return
		}
		const preInstalled = Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION)
		await expect(Compiler.installSolcVersion(ALT_SOLC_VERSION)).resolves.toBeUndefined()
		expect(Compiler.isSolcVersionInstalled(ALT_SOLC_VERSION)).toBe(true)
		if (!preInstalled) {
			await expect(Compiler.installSolcVersion(ALT_SOLC_VERSION)).resolves.toBeUndefined()
		}
	})

	test('isSolcVersionInstalled rejects malformed versions', () => {
		expect(() => Compiler.isSolcVersionInstalled('not-a-version')).toThrowErrorMatchingInlineSnapshot(
			`"Failed to parse solc version: unexpected character 'n' while parsing major version number"`,
		)
	})

	test('isSolcVersionInstalled respects custom svm home', () => {
		const original = process.env.SVM_HOME
		const temp = createTempDir('tevm-svm-')
		process.env.SVM_HOME = temp
		try {
			const overridden = Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)
			expect(typeof overridden).toBe('boolean')
		} finally {
			if (original === undefined) {
				delete process.env.SVM_HOME
			} else {
				process.env.SVM_HOME = original
			}
		}
	})
})

describe('Compiler constructor', () => {
	test('rejects invalid settings shape', () => {
		expect(() => new Compiler({ cacheEnabled: false, solcSettings: 42 as unknown as any })).toThrowErrorMatchingInlineSnapshot(
			`"solcSettings override must be provided as an object."`,
		)
	})

	test('rejects malformed solc versions at construction', () => {
		expect(() => new Compiler({ cacheEnabled: false, solcVersion: 'bad-version' })).toThrowErrorMatchingInlineSnapshot(
			`"Failed to parse solc version: unexpected character 'b' while parsing major version number"`,
		)
	})

	test('rejects when requested solc version is not installed', () => {
		expect(() => new Compiler({ cacheEnabled: false, solcVersion: '123.45.67' })).toThrowErrorMatchingInlineSnapshot(
			`"Solc 123.45.67 is not installed. Call installSolcVersion first."`,
		)
	})

	test('accepts nested settings without mutating defaults', () => {
	const compiler = new Compiler({
		cacheEnabled: false,
		solcVersion: DEFAULT_SOLC_VERSION,
		solcSettings: {
				optimizer: { enabled: true, runs: 9 },
				metadata: { bytecodeHash: BytecodeHash.None },
				debug: {
					revertStrings: RevertStrings.Debug,
					debugInfo: ['*'],
				},
				libraries: {
					'': {
						MathLib: `0x${'11'.repeat(20)}`,
					},
				},
				outputSelection: DEFAULT_OUTPUT_SELECTION,
				evmVersion: EvmVersion.London,
			},
		})

	const first = compiler.compileSource(INLINE_SOURCE)
	const second = compiler.compileSource(INLINE_SOURCE)

	for (const output of [first, second]) {
		expect(flattenContracts(output)).toHaveLength(1)
	}
	})

	test('per-call overrides leaving outputSelection empty are sanitized', () => {
	const compiler = new Compiler({ cacheEnabled: false })
	const first = compiler.compileSource(INLINE_SOURCE)
	const second = compiler.compileSource(INLINE_SOURCE, {
		solcSettings: {
			optimizer: { enabled: true, runs: 1 },
			outputSelection: {
				'*': { '*': [], '': [] },
			},
		},
	})
	const third = compiler.compileSource(INLINE_SOURCE)

	for (const output of [first, second, third]) {
		expect(flattenContracts(output)).toHaveLength(1)
	}
	expect(second.hasCompilerErrors()).toBe(false)
	})

	test('per-call solc version overrides do not leak into subsequent compiles', () => {
		const compiler = new Compiler({ cacheEnabled: false, solcVersion: DEFAULT_SOLC_VERSION })
		if (!altVersionInstalled) {
			const baseline = compiler.compileSource(INLINE_SOURCE)
			expect(baseline.hasCompilerErrors()).toBe(false)
			return
		}
		const baseline = compiler.compileSource(INLINE_SOURCE)
		const alt = compiler.compileSource(INLINE_SOURCE, {
			solcVersion: ALT_SOLC_VERSION,
		})
		const after = compiler.compileSource(INLINE_SOURCE)

		expect(baseline.hasCompilerErrors()).toBe(false)
		expect(alt.hasCompilerErrors()).toBe(false)
		expect(after.hasCompilerErrors()).toBe(false)
	})

	test('per-call overrides referencing missing solc versions throw and keep state intact', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		expect(() => compiler.compileSource(INLINE_SOURCE, { solcVersion: '999.0.0' })).toThrowErrorMatchingInlineSnapshot(
			`"Solc 999.0.0 is not installed. Call installSolcVersion first."`,
		)
		const result = compiler.compileSource(INLINE_SOURCE)
		expect(result.hasCompilerErrors()).toBe(false)
	})
})

describe('Compiler.compileSource with Solidity strings', () => {
	test('compiles inline solidity and exposes artifacts', () => {
	const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSource(INLINE_SOURCE)

	expect(output.hasCompilerErrors()).toBe(false)
	expect(output.errors).toBeUndefined()
	const json = output.toJson()
	const artifactContracts = Object.values(json.artifact?.contracts ?? {})
	const additionalContracts = Object.values(json.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	const allContracts = [...artifactContracts, ...additionalContracts]
	expect(allContracts).toHaveLength(1)
	const contract = allContracts[0]
	expect(contract.name).toBe('InlineExample')
	expectBytecodeShape(contract.creationBytecode)
	expectBytecodeShape(contract.deployedBytecode)
	expectAbiShape(contract.abi)
	if (contract.methodIdentifiers) {
		expect(typeof contract.methodIdentifiers).toBe('object')
	}
	if (contract.immutableReferences) {
		expect(typeof contract.immutableReferences).toBe('object')
	}
	})

	test('produces warnings without marking compilation as failed', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSource(WARNING_SOURCE)

		expect(output.hasCompilerErrors()).toBe(false)
		expect(output.errors).toBeUndefined()
		const warnings = output.diagnostics.filter((diagnostic) => diagnostic.severity === SeverityLevel.Warning)
		expect(warnings.length).toBeGreaterThan(0)
		const severities = new Set(output.diagnostics.map((err) => err.severity))
		expect(severities.has(SeverityLevel.Warning)).toBe(true)
	})

	test('surfaces syntax errors without throwing', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSource(BROKEN_SOURCE)

		expect(output.hasCompilerErrors()).toBe(true)
		expect(output.errors).toBeDefined()
		const errors = output.errors ?? []
		expect(errors.length).toBeGreaterThan(0)
		const error = errors[0]
		expect(error.message).toMatch(/expected ';'/i)
		expect(error.severity).toBe(SeverityLevel.Error)
	})

	test('supports stopAfter parsing while keeping diagnostics', () => {
		const compiler = new Compiler({ cacheEnabled: false })
	const parsingOnly = compiler.compileSource(BROKEN_SOURCE, {
		solcSettings: { stopAfter: 'parsing' },
	})
	const parsingOnlyJson = parsingOnly.toJson()
	const parsingOnlyContracts = Object.values(parsingOnlyJson.artifact?.contracts ?? {})
	const parsingOnlyExtra = Object.values(parsingOnlyJson.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	expect([...parsingOnlyContracts, ...parsingOnlyExtra]).toHaveLength(0)
	expect(parsingOnly.hasCompilerErrors()).toBe(true)
		expect(parsingOnly.errors).toBeDefined()
		expect(parsingOnly.errors?.[0]?.message).toMatchInlineSnapshot(
			`"Requested output selection conflicts with "settings.stopAfter"."`,
		)

	const parsingOnlyCorrect = compiler.compileSource(INLINE_SOURCE, {
		solcSettings: {
			stopAfter: 'parsing',
			outputSelection: {
				'*': {
					'': ['ast'],
					},
				},
			},
		})
	const parsingOnlyCorrectJson = parsingOnlyCorrect.toJson()
	const parsingOnlyCorrectContracts = Object.values(parsingOnlyCorrectJson.artifact?.contracts ?? {})
	const parsingOnlyCorrectExtra = Object.values(parsingOnlyCorrectJson.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	expect([...parsingOnlyCorrectContracts, ...parsingOnlyCorrectExtra]).toHaveLength(0)
		expect(parsingOnlyCorrect.hasCompilerErrors()).toBe(false)
		expect(parsingOnlyCorrect.artifact?.ast).toBeDefined()
		expect(parsingOnlyCorrect.artifact?.contracts).toBeDefined()
		expect(Object.keys(parsingOnlyCorrect.artifact?.contracts ?? {})).toHaveLength(0)
	})

	test('accepts complete solcSettings payload', () => {
		const settings = {
			stopAfter: 'parsing',
			remappings: ['lib/=lib'],
			optimizer: { enabled: true, runs: 123, details: { yul: true } },
			modelChecker: {
				engine: ModelCheckerEngine.Bmc,
				timeout: 1,
				contracts: { '*': ['*'] },
			},
			metadata: {
				useLiteralContent: true,
				bytecodeHash: BytecodeHash.None,
				cborMetadata: false,
			},
			outputSelection: {
				'*': { '*': ['abi', 'evm.bytecode.object'] },
			},
			evmVersion: EvmVersion.Prague,
			viaIr: true,
			debug: { revertStrings: RevertStrings.Debug, debugInfo: ['location'] },
			libraries: {
				'LibraryConsumer.sol': {
					MathLib: '0x0000000000000000000000000000000000000001',
				},
			},
		} as const satisfies CompilerSettings

	const compiler = new Compiler({ cacheEnabled: false, solcSettings: settings })
	const output = compiler.compileSource(BROKEN_SOURCE, {
		solcSettings: settings,
	})

	const json = output.toJson()
	const contracts = Object.values(json.artifact?.contracts ?? {})
	const extraContracts = Object.values(json.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	expect([...contracts, ...extraContracts]).toHaveLength(0)
		expect(output.hasCompilerErrors()).toBe(true)
		expect(output.errors).toBeDefined()
		expect((output.errors ?? []).length).toBeGreaterThan(0)
	})

	test('respects per-call optimizer overrides', () => {
		const compiler = new Compiler({
			cacheEnabled: false,
			solcSettings: {
				optimizer: { enabled: false },
			},
		})

	const withoutOptimizer = compiler.compileSource(INLINE_SOURCE)
	const withOptimizer = compiler.compileSource(INLINE_SOURCE, {
		solcSettings: {
			optimizer: { enabled: true, runs: 200 },
		},
	})

	for (const output of [withoutOptimizer, withOptimizer]) {
		expect(flattenContracts(output)).toHaveLength(1)
	}
	expect(withOptimizer.errors).toBeUndefined()
	})

	test('allows metadata and evm version overrides', () => {
	const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSource(INLINE_SOURCE, {
		solcSettings: {
			metadata: { bytecodeHash: BytecodeHash.None },
			evmVersion: EvmVersion.London,
		},
	})
	expect(output.hasCompilerErrors()).toBe(false)
	const contracts = flattenContracts(output)
	expect(contracts).toHaveLength(1)
	})

	test('compiles multiple contracts in a single source', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSource(MULTI_CONTRACT_SOURCE)
		const contracts = flattenContracts(output)
		const names = contracts.map((contract) => contract.name)
		expect(names).toEqual(expect.arrayContaining(['First', 'Second', 'Target']))
	})

	test('supports concurrent compilation calls', async () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const [a, b] = await Promise.all([
			Promise.resolve().then(() => compiler.compileSource(INLINE_SOURCE)),
			Promise.resolve().then(() => compiler.compileSource(MULTI_CONTRACT_SOURCE)),
		])

		expect(a.hasCompilerErrors()).toBe(false)
		expect(b.hasCompilerErrors()).toBe(false)
		expect(flattenContracts(a)).toHaveLength(1)
		expect(flattenContracts(b)).toHaveLength(3)
	})
})

describe('Compiler.compileSource with AST and Yul inputs', () => {
	test('accepts pre-parsed AST values', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
	const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSource(ast)
	expect(output.hasCompilerErrors()).toBe(false)
	const [contract] = flattenContracts(output)
	expect(contract.name).toBe('InlineExample')
	})

	test('returns diagnostics when AST lacks contract definitions', () => {
	const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSource(deepClone(EMPTY_SOURCE_UNIT))
	expect(output.hasCompilerErrors()).toBe(false)
	expect(flattenContracts(output)).toHaveLength(0)
		expect(output.errors).toBeUndefined()
		expect(Array.isArray(output.diagnostics)).toBe(true)
	})

	test('compiles sanitized AST after instrumentation', () => {
	const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
		.fromSource(INLINE_SOURCE)
		.injectShadow(FUNCTION_FRAGMENT)
		.injectShadow(VARIABLE_FRAGMENT)
		.ast()

	const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSource(instrumented)
	expect(output.hasCompilerErrors()).toBe(false)
	const [contract] = flattenContracts(output)
	expect(contract.name).toBe('InlineExample')
	})

	test('rejects unsupported languages for AST sources', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
		const compiler = new Compiler({ cacheEnabled: false })
		expect(() =>
			compiler.compileSource(ast, {
				language: CompilerLanguage.Yul,
			}),
		).toThrow(/AST compilation is only supported for Solidity sources/i)
		expect(() =>
			compiler.compileSource(ast, {
				language: CompilerLanguage.Vyper,
			}),
		).toThrow(/AST compilation is only supported for Solidity sources/i)
	})

	test('compiles Yul sources when requested', () => {
		const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSource(YUL_SOURCE, {
		language: CompilerLanguage.Yul,
	})
	expect(output.hasCompilerErrors()).toBe(false)
	const json = output.toJson()
	const contracts = Object.values(json.artifact?.contracts ?? {})
	const extraContracts = Object.values(json.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	const allContracts = [...contracts, ...extraContracts]
	expect(allContracts).toHaveLength(1)
	const compiled = allContracts[0]
	expectBytecodeShape(compiled.creationBytecode)
	expectBytecodeShape(compiled.deployedBytecode)
	})

	test('compiles Vyper sources when requested', () => {
		const compiler = new Compiler({ cacheEnabled: false, language: CompilerLanguage.Vyper })
	const output = compiler.compileSource(VYPER_COUNTER_SOURCE, {
		language: CompilerLanguage.Vyper,
	})
	expect(output.hasCompilerErrors()).toBe(false)
	const json = output.toJson()
	const contracts = Object.values(json.artifact?.contracts ?? {})
	const extraContracts = Object.values(json.artifacts ?? {}).flatMap((source) =>
		Object.values(source?.contracts ?? {}),
	)
	expect([...contracts, ...extraContracts]).toHaveLength(1)
	})
})

describe('Compiler.compileSources', () => {
	test('compiles multiple solidity entries by path', () => {
		const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSources({
		'InlineExample.sol': INLINE_SOURCE,
		'WarningContract.sol': WARNING_SOURCE,
	})

	const names = flattenContracts(output).map((contract) => contract.name)
	expect(names).toEqual(expect.arrayContaining(['InlineExample', 'WarningContract']))
	})

	test('compiles Yul sources when supplied as a map', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSources(
			{
				'Echo.yul': YUL_SOURCE,
			},
			{ language: CompilerLanguage.Yul },
		)

		expect(output.hasCompilerErrors()).toBe(false)
		expect(flattenContracts(output)).toHaveLength(1)
	})

	test('compiles Vyper sources when supplied as a map', () => {
		const compiler = new Compiler({ cacheEnabled: false, language: CompilerLanguage.Vyper })
		const output = compiler.compileSources({
			'Counter.vy': VYPER_COUNTER_SOURCE,
		})
		expect(output.hasCompilerErrors()).toBe(false)
		expect(flattenContracts(output)).toHaveLength(1)
	})

	test('compiles AST entries keyed by path', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSources({ 'InlineExample.sol': ast })

		expect(output.hasCompilerErrors()).toBe(false)
	const [contract] = flattenContracts(output)
	expect(contract.name).toBe('InlineExample')
	})

	test('rejects mixing ast and source strings', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
		const compiler = new Compiler({ cacheEnabled: false })
		expect(() =>
			compiler.compileSources({
				'InlineExample.sol': INLINE_SOURCE,
				'InlineExample.ast': ast,
			}),
		).toThrowErrorMatchingInlineSnapshot(
			`"compileSources does not support mixing inline source strings with AST entries in the same call."`,
		)
	})
})

describe('Compiler toJson snapshots', () => {
	test('captures structured Solidity artifacts', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSources({
			'InlineExample.sol': INLINE_SOURCE,
		})
		expect(output.toJson()).toMatchSnapshot()
	})

	test('captures structured Yul artifacts', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileSources(
			{
				'Echo.yul': YUL_SOURCE,
			},
			{ language: CompilerLanguage.Yul },
		)
		expect(output.toJson()).toMatchSnapshot()
	})

	test('captures structured Vyper artifacts', () => {
		const compiler = new Compiler({ cacheEnabled: false, language: CompilerLanguage.Vyper })
		const output = compiler.compileSources({
			'Counter.vy': VYPER_COUNTER_SOURCE,
		})
		expect(output.toJson()).toMatchSnapshot()
	})
})

describe('Compiler.compileFiles', () => {
	test('compiles solidity files from disk', () => {
		const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileFiles([INLINE_PATH, WARNING_PATH])

	const names = flattenContracts(output).map((contract) => contract.name)
	expect(names).toEqual(expect.arrayContaining(['InlineExample', 'WarningContract']))
	})

	test('compiles yul files when language override is provided', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		const output = compiler.compileFiles([YUL_PATH], {
			language: CompilerLanguage.Yul,
		})

		expect(output.hasCompilerErrors()).toBe(false)
		expect(flattenContracts(output)).toHaveLength(1)
	})

	test('compiles vyper files when language override is provided', () => {
		const compiler = new Compiler({ cacheEnabled: false, language: CompilerLanguage.Vyper })
		const output = compiler.compileFiles([VYPER_COUNTER_PATH], {
			language: CompilerLanguage.Vyper,
		})
		expect(output.hasCompilerErrors()).toBe(false)
		expect(flattenContracts(output)).toHaveLength(1)
	})

	test('throws when a path cannot be read', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		expect(() => compiler.compileFiles(['/non-existent/path.sol'])).toThrowErrorMatchingInlineSnapshot(
			`"Failed to read source file: No such file or directory (os error 2)"`,
		)
	})

	test('compiles json ast files', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
		const dir = createTempDir('tevm-compile-files-ast-')
		const astPath = join(dir, 'InlineExample.ast.json')
		writeFileSync(astPath, JSON.stringify(ast))

		const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileFiles([astPath])

	expect(output.hasCompilerErrors()).toBe(false)
	const [contract] = flattenContracts(output)
	expect(contract.name).toBe('InlineExample')
	})

	test('compiles ast files with unrecognized extensions', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
		const dir = createTempDir('tevm-compile-files-ast-ext-')
		const astPath = join(dir, 'InlineExample.ast')
		writeFileSync(astPath, JSON.stringify(ast))

		const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileFiles([astPath])

	const [contract] = flattenContracts(output)
	expect(contract.name).toBe('InlineExample')
	})

	test('errors when mixing ast and source inputs', () => {
	const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).ast()
		const dir = createTempDir('tevm-compile-files-mix-')
		const astPath = join(dir, 'InlineExample.ast.json')
		writeFileSync(astPath, JSON.stringify(ast))
		const compiler = new Compiler({ cacheEnabled: false })

		expect(() => compiler.compileFiles([INLINE_PATH, astPath])).toThrowErrorMatchingInlineSnapshot(
			`"compileSources does not support mixing inline source strings with AST entries in the same call."`,
		)
	})

	test('errors when extension is unknown and no language override is provided', () => {
		const dir = createTempDir('tevm-compile-files-unknown-')
		const unknownPath = join(dir, 'InlineExample.txt')
		writeFileSync(unknownPath, INLINE_SOURCE)
		const compiler = new Compiler({ cacheEnabled: false })

		expect(() => compiler.compileFiles([unknownPath])).toThrow(/Unable to infer compiler language/i)
	})

	test('errors when multiple languages are detected', () => {
		const compiler = new Compiler({ cacheEnabled: false })
		expect(() => compiler.compileFiles([INLINE_PATH, YUL_PATH])).toThrowErrorMatchingInlineSnapshot(
			`"compileFiles requires all non-AST sources to share the same language. Provide language explicitly to disambiguate."`,
		)
	})

	test('ignores constructor language preference', () => {
		const compiler = new Compiler({
			cacheEnabled: false,
			solcVersion: DEFAULT_SOLC_VERSION,
			language: CompilerLanguage.Yul,
		})
		const output = compiler.compileFiles([INLINE_PATH])

		expect(output.hasCompilerErrors()).toBe(false)
		const [contract] = flattenContracts(output)
		expect(contract.name).toBe('InlineExample')
	})

	test('rejects json files that are not objects', () => {
		const dir = createTempDir('tevm-compile-files-json-')
		const jsonPath = join(dir, 'Invalid.json')
		writeFileSync(jsonPath, '[]')
		const compiler = new Compiler({ cacheEnabled: false })

		expect(() => compiler.compileFiles([jsonPath])).toThrowErrorMatchingInlineSnapshot(
			`"JSON sources must contain a Solidity AST object."`,
		)
	})
})

describe('Compiler project paths', () => {
	test('reports synthetic layout when no project is attached', () => {
		const root = createTempDir('tevm-synth-')
		const compiler = Compiler.fromRoot(root, { cacheEnabled: false })
		const paths = compiler.getPaths()
		const canonical = realpathSync(root)

		expect(paths.root).toBe(canonical)
		expect(paths.cache).toBe(join(canonical, '.tevm', 'cache', 'solidity-files-cache.json'))
		expect(paths.artifacts).toBe(join(canonical, '.tevm', 'out'))
		expect(paths.buildInfos).toBe(join(canonical, '.tevm', 'out', 'build-info'))
		expect(paths.sources).toBe(canonical)
		expect(paths.tests).toBe(join(canonical, 'test'))
		expect(paths.scripts).toBe(join(canonical, 'scripts'))
		expect(paths.virtualSources).toBe(join(canonical, '.tevm', 'virtual-sources'))
		expect(paths.libraries).toHaveLength(0)
		expect(paths.includePaths).toHaveLength(0)
		expect(new Set(paths.allowedPaths)).toContain(canonical)
	})

	test('writes cache artifacts for inline sources in default synthetic workspace', () => {
		const workspace = createTempDir('tevm-default-cache-')
		const script = `
const { Compiler } = require(${JSON.stringify(COMPILER_MODULE_PATH)});
const source = ${JSON.stringify(INLINE_SOURCE)};
const compiler = new Compiler();
const output = compiler.compileSource(source);
if (output.hasCompilerErrors()) {
	throw new Error('Compilation produced errors');
}
`
		const result = spawnSync(process.execPath, ['-e', script], {
			cwd: workspace,
			encoding: 'utf8',
		})

		if (result.status !== 0) {
			throw new Error(result.stderr || 'Failed to execute compilation script')
		}

		const tevmRoot = join(workspace, '.tevm')
		const cacheFile = join(tevmRoot, 'cache', 'solidity-files-cache.json')
		const artifactsDir = join(tevmRoot, 'out')
		const virtualSources = join(tevmRoot, 'virtual-sources')

		expect(existsSync(cacheFile)).toBe(true)
		expect(listJsonFiles(artifactsDir).some((file) => !file.includes('build-info'))).toBe(true)
		expect(existsSync(virtualSources)).toBe(true)
		const virtualEntries = readdirSync(virtualSources)
		expect(virtualEntries.some((entry) => entry.endsWith('.sol'))).toBe(true)
	})

	test('writes cache artifacts for inline sources in synthetic project', () => {
		const root = createTempDir('tevm-synth-cache-')
		const compiler = Compiler.fromRoot(root, { cacheEnabled: false })
		const tevmRoot = join(root, '.tevm')
		const cacheFile = join(tevmRoot, 'cache', 'solidity-files-cache.json')
		const artifactsDir = join(tevmRoot, 'out')
		const virtualSources = join(tevmRoot, 'virtual-sources')

		const output = compiler.compileSource(INLINE_SOURCE)

		expect(output.hasCompilerErrors()).toBe(false)
		expect(existsSync(cacheFile)).toBe(true)
		expect(JSON.parse(readFileSync(cacheFile, 'utf8'))).toBeTruthy()
		expect(listJsonFiles(artifactsDir).some((file) => !file.includes('build-info'))).toBe(true)
		expect(existsSync(virtualSources)).toBe(true)
		const virtualEntries = readdirSync(virtualSources)
		expect(virtualEntries.some((entry) => entry.endsWith('.sol'))).toBe(true)
	})
})
