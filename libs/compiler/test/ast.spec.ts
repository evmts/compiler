import { beforeAll, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { Ast, Compiler } from '../build/index.js'
import type { ContractDefinition, FunctionDefinition, SourceUnit } from '../build/solc-ast.js'

const DEFAULT_SOLC_VERSION = '0.8.30'
const FIXTURES_DIR = join(__dirname, 'fixtures')
const CONTRACTS_DIR = join(FIXTURES_DIR, 'contracts')
const FRAGMENTS_DIR = join(FIXTURES_DIR, 'fragments')
const AST_DIR = join(FIXTURES_DIR, 'ast')

const INLINE_SOURCE = readFileSync(join(CONTRACTS_DIR, 'InlineExample.sol'), 'utf8')
const MULTI_CONTRACT_SOURCE = readFileSync(join(CONTRACTS_DIR, 'MultiContract.sol'), 'utf8')
const NO_CONTRACTS_SOURCE = readFileSync(join(CONTRACTS_DIR, 'NoContracts.sol'), 'utf8')
const FUNCTION_FRAGMENT = readFileSync(join(FRAGMENTS_DIR, 'function_fragment.sol'), 'utf8')
const FUNCTION_FRAGMENT_OVERRIDE = readFileSync(join(FRAGMENTS_DIR, 'function_fragment_override.sol'), 'utf8')
const VARIABLE_FRAGMENT = readFileSync(join(FRAGMENTS_DIR, 'variable_fragment.sol'), 'utf8')
const SHADOW_CONTRACT_FRAGMENT = readFileSync(join(FRAGMENTS_DIR, 'shadow_contract.sol'), 'utf8')
const EMPTY_SOURCE_UNIT = JSON.parse(readFileSync(join(AST_DIR, 'empty_source_unit.json'), 'utf8'))
const FRAGMENT_WITHOUT_TARGET = JSON.parse(readFileSync(join(AST_DIR, 'fragment_without_contract.json'), 'utf8'))

let sharedCompiler: Compiler

const findContract = (unit: SourceUnit, name: string): ContractDefinition | undefined =>
	unit.nodes
		.filter((node) => node.nodeType === 'ContractDefinition')
		.map((node) => node as unknown as ContractDefinition)
		.find((definition) => definition.name === name)

const findFunction = (unit: SourceUnit, contractName: string, functionName: string): FunctionDefinition | undefined => {
	const contract = findContract(unit, contractName)
	if (!contract) return undefined
	return contract.nodes.find(
		(node): node is FunctionDefinition => node.nodeType === 'FunctionDefinition' && node.name === functionName,
	)
}

const collectIds = (value: unknown, ids: number[]) => {
	if (Array.isArray(value)) {
		value.forEach((child) => {
			collectIds(child, ids)
		})
		return
	}
	if (value && typeof value === 'object') {
		const record = value as Record<string, unknown>
		if (typeof record.id === 'number') {
			ids.push(record.id)
		}
		Object.values(record).forEach((child) => {
			collectIds(child, ids)
		})
	}
}

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value))

const normaliseArtifacts = (output: any) => {
	const result: Record<string, any> = {}
	const primary = output.artifact
	if (primary) {
		const key = primary.sourcePath ?? output.primarySource ?? '__virtual__'
		result[key] = primary
	}
	for (const [sourceName, sourceArtifacts] of Object.entries(output.artifacts ?? {})) {
		result[sourceName] = sourceArtifacts
	}
	return result
}

const collectContracts = (output: any) => {
	return Object.entries(normaliseArtifacts(output)).flatMap(([sourceName, sourceArtifacts]) =>
		Object.entries((sourceArtifacts as any).contracts ?? {}).map(([contractName, contract]) => {
			const resolved = contract as any
			const name = resolved?.name ?? contractName
			return {
				sourceName,
				contractName: name,
				artifact: resolved,
			}
		}),
	)
}

const findTapStored = (unit: SourceUnit) => {
	const contract = findContract(unit, 'InlineExample')
	if (!contract) {
		throw new Error('InlineExample contract not found in unit')
	}
	const functionNode = contract.nodes.find(
		(node): node is any => node.nodeType === 'FunctionDefinition' && (node as any).name === 'tapStored',
	)
	if (!functionNode) {
		throw new Error('tapStored function not present in contract')
	}
	return functionNode
}

beforeAll(() => {
	if (!Compiler.isSolcVersionInstalled(DEFAULT_SOLC_VERSION)) {
		throw new Error(
			`Solc ${DEFAULT_SOLC_VERSION} must be installed before running ast tests. ` +
				`Install it via Compiler.installSolcVersion or Foundry's svm ahead of time.`,
		)
	}
	sharedCompiler = new Compiler({ solcVersion: DEFAULT_SOLC_VERSION })
})

describe('Ast constructor', () => {
	test('creates instances with default configuration', () => {
		const ast = new Ast()
		expect(ast).toBeInstanceOf(Ast)
	})

	test('rejects malformed settings objects', () => {
		expect(() => new Ast({ solcSettings: 42 as unknown as any })).toThrowErrorMatchingInlineSnapshot(
			`"solcSettings override must be provided as an object."`,
		)
	})

	test('rejects unsupported solc language overrides', () => {
		expect(() => new Ast({ solcLanguage: 'Yul' as any })).toThrowErrorMatchingInlineSnapshot(
			`"Ast helpers only support solcLanguage "Solidity"."`,
		)
	})

	test('rejects when requested solc version is not installed', () => {
		expect(() => new Ast({ solcVersion: '999.0.0' })).toThrowErrorMatchingInlineSnapshot(
			`"Solc 999.0.0 is not installed. Call installSolcVersion first."`,
		)
	})
})

describe('fromSource', () => {
	test('hydrates from source string', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE)
		const ast = instrumented.sourceUnit()

		const contract = findContract(ast, 'InlineExample')
		expect(contract).toBeTruthy()
	})

	test('hydrates from existing ast values', () => {
		const sourceAst = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).sourceUnit()
		const roundTripped = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(sourceAst).sourceUnit()
		expect(roundTripped).toEqual(sourceAst)
	})

	test('applies instrumentedContract overrides per call', () => {
		const instrumented = new Ast({
			solcVersion: DEFAULT_SOLC_VERSION,
			instrumentedContract: 'Target',
		}).fromSource(MULTI_CONTRACT_SOURCE)
		const ast = instrumented.sourceUnit()
		const target = findContract(ast, 'Target')
		const second = findContract(ast, 'Second')

		expect(target).toBeTruthy()
		expect(second).toBeTruthy()
	})

	test('throws when ast is requested before initialization', () => {
		const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
		expect(() => ast.sourceUnit()).toThrowErrorMatchingInlineSnapshot(
			`"Ast has no target unit. Call from_source first."`,
		)
	})

	test('handles missing contracts when instrumented contract is configured', () => {
		const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION, instrumentedContract: 'Missing' }).fromSource(
			NO_CONTRACTS_SOURCE,
		)
		const unit = ast.sourceUnit()
		const contracts = unit.nodes.filter((node) => node.nodeType === 'ContractDefinition')
		expect(contracts).toHaveLength(0)
	})
})

describe('injectShadow', () => {
	test('injects fragment functions from source strings', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)

		const contract = findContract(instrumented.sourceUnit(), 'InlineExample')
		const functionNames = contract!.nodes
			.filter((node) => node.nodeType === 'FunctionDefinition')
			.map((fn: any) => fn.name)
		expect(functionNames).toContain('tapStored')
	})

	test('injects fragment variables sequentially and keeps ids unique', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)
			.injectShadow(VARIABLE_FRAGMENT)
			.validate()

		const ast = instrumented.sourceUnit()
		const ids: number[] = []
		collectIds(ast, ids)
		expect(ids.length).toBeGreaterThan(0)
		expect(ids.length).toBe(new Set(ids).size)
	})

	test('injects pre-parsed ast fragments', () => {
		const fragmentAst = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(SHADOW_CONTRACT_FRAGMENT).sourceUnit()
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(fragmentAst)
			.validate()

		const contract = findContract(instrumented.sourceUnit(), 'InlineExample')
		const functionNames = contract!.nodes
			.filter((node) => node.nodeType === 'FunctionDefinition')
			.map((fn: any) => fn.name)
		expect(functionNames).toContain('shadowy')
	})

	test('rejects fragments without __AstFragment contract', () => {
		const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE)
		expect(() => ast.injectShadow(clone(FRAGMENT_WITHOUT_TARGET))).toThrowErrorMatchingInlineSnapshot(
			`"Failed to locate fragment contract: Parse failed: Fragment contract '__AstFragment' not found"`,
		)
	})

	test('rejects injection before loading a source', () => {
		const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
		expect(() => ast.injectShadow(FUNCTION_FRAGMENT)).toThrowErrorMatchingInlineSnapshot(
			`"Ast has no target AST. Call from_source first."`,
		)
	})

	test('defaults to safe conflict resolution when members clash', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)
			.injectShadow(FUNCTION_FRAGMENT_OVERRIDE)

		const contract = findContract(instrumented.sourceUnit(), 'InlineExample')!
		const tapStored = contract.nodes.filter(
			(node): node is any => node.nodeType === 'FunctionDefinition' && (node as any).name === 'tapStored',
		)
		expect(tapStored).toHaveLength(2)
		expect(() => instrumented.validate()).toThrow('Analysis of the AST failed')
	})

	test('replace conflict strategy swaps existing members in place', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)

		const original = findTapStored(instrumented.sourceUnit())
		const originalId = original.id

		instrumented.injectShadow(FUNCTION_FRAGMENT_OVERRIDE, {
			resolveConflictStrategy: 'replace',
		})

		const contract = findContract(instrumented.sourceUnit(), 'InlineExample')!
		const tapStored = contract.nodes.filter(
			(node): node is any => node.nodeType === 'FunctionDefinition' && (node as any).name === 'tapStored',
		)
		expect(tapStored).toHaveLength(1)
		expect(tapStored[0].id).toBe(originalId)
		expect(JSON.stringify(tapStored[0].body)).toContain('"42"')

		const replacementVariable = contract.nodes.find(
			(node): node is any => node.nodeType === 'VariableDeclaration' && (node as any).name === 'replacementCounter',
		)
		expect(replacementVariable).toBeDefined()

		const ids: number[] = []
		collectIds(instrumented.sourceUnit(), ids)
		expect(ids.length).toBe(new Set(ids).size)
		instrumented.validate()
	})
})

describe('injectShadowAtEdges', () => {
	const requireCallMatcher = (statement: any) =>
		statement?.nodeType === 'ExpressionStatement' &&
		statement?.expression?.nodeType === 'FunctionCall' &&
		statement?.expression?.expression?.name === 'require'

	test('injects before and after statements', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadowAtEdges('get()', {
				before: 'uint256 __checkpoint = stored;',
				after: 'require(stored >= __checkpoint);',
			})
			.validate()

		const fn = findFunction(instrumented.sourceUnit(), 'InlineExample', 'get')
		expect(fn).toBeDefined()
		const statements = fn?.body?.statements ?? []
		expect(statements).toHaveLength(4)

		const [first, second, third, fourth] = statements
		expect(first).toMatchObject({
			nodeType: 'VariableDeclarationStatement',
			declarations: [expect.objectContaining({ name: '__checkpoint', nodeType: 'VariableDeclaration' })],
			initialValue: expect.objectContaining({
				nodeType: 'Identifier',
				name: 'stored',
			}),
		})
		expect(requireCallMatcher(second)).toBe(true)
		expect(third?.nodeType).toBe('Return')
		expect(requireCallMatcher(fourth)).toBe(true)
	})

	test('accepts snippet arrays', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadowAtEdges('get()', {
				before: ['uint256 __checkpoint = stored;', 'uint256 __second = stored;'],
				after: ['require(__second >= __checkpoint);'],
			})
			.validate()

		const fn = findFunction(instrumented.sourceUnit(), 'InlineExample', 'get')
		expect(fn).toBeDefined()
		const statements = fn?.body?.statements ?? []
		expect(statements).toHaveLength(5)
		expect(statements[0]).toMatchObject({
			nodeType: 'VariableDeclarationStatement',
			declarations: [expect.objectContaining({ name: '__checkpoint' })],
		})
		expect(statements[1]).toMatchObject({
			nodeType: 'VariableDeclarationStatement',
			declarations: [expect.objectContaining({ name: '__second' })],
		})
		expect(requireCallMatcher(statements[2])).toBe(true)
		expect(statements[3]?.nodeType).toBe('Return')
		expect(requireCallMatcher(statements[4])).toBe(true)
	})

	test('throws when snippets are missing', () => {
		expect(() =>
			new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).injectShadowAtEdges('get()', {}),
		).toThrowErrorMatchingInlineSnapshot(`"injectShadowAtEdges requires a \`before\` and/or \`after\` snippet."`)
	})

	test('throws when selector is ambiguous', () => {
		expect(() =>
			new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
				.fromSource(`
			contract Overloads {
				function call(uint256 value) public pure returns (uint256) { return value; }
				function call(address target) public pure returns (address) { return target; }
			}
		`)
				.injectShadowAtEdges('call', { before: 'uint256 sentry = 1;' }),
		).toThrowErrorMatchingInlineSnapshot(`"Function name is ambiguous. Please provide a full function signature."`)
	})

	test('throws when function is missing', () => {
		expect(() =>
			new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
				.fromSource(INLINE_SOURCE)
				.injectShadowAtEdges('missing()', { before: 'uint256 sentinel = 0;' }),
		).toThrowErrorMatchingInlineSnapshot(`"Target function not found for injectShadowAtEdges."`)
	})
})

describe('validate', () => {
	test('recompiles the AST to populate resolved type information', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)

		const parsedUnit = instrumented.sourceUnit()
		const parsedTapStored = findTapStored(parsedUnit)
		const parsedTypeDescriptions = parsedTapStored.returnParameters.parameters[0].typeDescriptions ?? {}
		expect(Object.keys(parsedTypeDescriptions)).toHaveLength(0)

		const validatedUnit = instrumented.validate().sourceUnit()
		const validatedTapStored = findTapStored(validatedUnit)
		const validatedTypeDescriptions = validatedTapStored.returnParameters.parameters[0].typeDescriptions

		expect(validatedTypeDescriptions).toMatchObject({
			typeIdentifier: expect.stringMatching(/^t_uint256/),
			typeString: 'uint256',
		})
	})
})

describe('compile', () => {
	test('returns compile output snapshot', () => {
		const output = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(INLINE_SOURCE).compile()

		expect(output.hasCompilerErrors()).toBe(false)
		expect(output.artifact?.toJson()).toMatchSnapshot()
	})
})

describe('visibility transformations', () => {
	test('promotes private and internal variables to public', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(MULTI_CONTRACT_SOURCE, { instrumentedContract: 'Target' })
			.exposeInternalVariables({ instrumentedContract: 'Target' })

		const target = findContract(instrumented.sourceUnit(), 'Target')!
		const visibilities = target.nodes
			.filter((node) => node.nodeType === 'VariableDeclaration')
			.map((node: any) => node.visibility)
		expect(new Set(visibilities)).toEqual(new Set(['public']))
	})

	test('promotes private and internal functions to public', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(MULTI_CONTRACT_SOURCE, { instrumentedContract: 'Target' })
			.exposeInternalFunctions({ instrumentedContract: 'Target' })

		const target = findContract(instrumented.sourceUnit(), 'Target')!
		const visibilities = target.nodes
			.filter((node) => node.nodeType === 'FunctionDefinition')
			.map((node: any) => node.visibility)
		expect(visibilities).toContain('public')
	})

	test('applies visibility changes across all contracts when no override is provided', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(MULTI_CONTRACT_SOURCE)
			.exposeInternalVariables()
			.exposeInternalFunctions()

		const ast = instrumented.sourceUnit()
		const first = findContract(ast, 'First')!
		const second = findContract(ast, 'Second')!
		const target = findContract(ast, 'Target')!

		const firstVars = first.nodes
			.filter((node) => node.nodeType === 'VariableDeclaration')
			.map((node: any) => node.visibility)
		const secondVars = second.nodes
			.filter((node) => node.nodeType === 'VariableDeclaration')
			.map((node: any) => node.visibility)
		const targetFuncs = target.nodes
			.filter((node) => node.nodeType === 'FunctionDefinition')
			.map((node: any) => node.visibility)

		expect(new Set(firstVars)).toEqual(new Set(['public']))
		expect(new Set(secondVars)).toEqual(new Set(['public']))
		expect(targetFuncs).toContain('public')
	})

	test('rejects visibility changes before loading a source', () => {
		const ast = new Ast({ solcVersion: DEFAULT_SOLC_VERSION, instrumentedContract: 'Target' })
		expect(() => ast.exposeInternalVariables()).toThrowErrorMatchingInlineSnapshot(
			`"Ast has no target AST. Call from_source first."`,
		)
		expect(() => ast.exposeInternalFunctions()).toThrowErrorMatchingInlineSnapshot(
			`"Ast has no target AST. Call from_source first."`,
		)
	})

	test('throws when targeted contract is missing during visibility updates', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION }).fromSource(MULTI_CONTRACT_SOURCE)
		expect(() =>
			instrumented.exposeInternalVariables({ instrumentedContract: 'Missing' }),
		).toThrowErrorMatchingInlineSnapshot(`"Invalid contract structure: Contract 'Missing' not found"`)
	})
})

describe('integration with Compiler', () => {
	test('compiled instrumented ast executes without diagnostics', () => {
		const instrumented = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)
			.injectShadow(VARIABLE_FRAGMENT)
			.exposeInternalVariables()
			.exposeInternalFunctions()

		const ast = instrumented.sourceUnit()
		const output = sharedCompiler.compileSource(ast)

		expect(output.hasCompilerErrors()).toBe(false)
		expect(collectContracts(output)[0]?.contractName).toBe('InlineExample')
	})

	test('handles ast inputs without contracts gracefully', () => {
		const output = sharedCompiler.compileSource(clone(EMPTY_SOURCE_UNIT))
		expect(collectContracts(output)).toHaveLength(0)
		expect(output.errors).toBeUndefined()
		expect(Array.isArray(output.diagnostics)).toBe(true)
	})

	test('sourceUnit() returns sanitized json without null entries', () => {
		const sourceUnit = new Ast({ solcVersion: DEFAULT_SOLC_VERSION })
			.fromSource(INLINE_SOURCE)
			.injectShadow(FUNCTION_FRAGMENT)
			.sourceUnit()
		const serialized = JSON.stringify(sourceUnit)
		expect(serialized.includes('null')).toBe(false)
	})
})
