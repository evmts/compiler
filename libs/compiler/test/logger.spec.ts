import { afterEach, beforeEach, describe, expect, test } from 'bun:test'
import { join } from 'node:path'
import { Ast, Compiler, Contract } from '../build/index.js'

const INLINE_SOURCE = 'contract Logger { function noop() public {} }'
const _INVALID_SOURCE = 'contract Broken {'

const flushLogs = async () => {
	await new Promise((resolve) => setTimeout(resolve, 100))
}

const originalConsole = {
	log: console.log,
	warn: console.warn,
	error: console.error,
}

let capturedLog: string[] = []
let capturedWarn: string[] = []
let capturedError: string[] = []

const capture =
	(bucket: string[]) =>
	(...args: unknown[]) => {
		const message = args.map((value) => String(value)).join(' ')
		bucket.push(message)
	}

describe('Logger integration', () => {
	beforeEach(() => {
		capturedLog = []
		capturedWarn = []
		capturedError = []
		console.log = capture(capturedLog)
		console.warn = capture(capturedWarn)
		console.error = capture(capturedError)
	})

	afterEach(() => {
		console.log = originalConsole.log
		console.warn = originalConsole.warn
		console.error = originalConsole.error
		capturedLog = []
		capturedWarn = []
		capturedError = []
	})

	test('compiler emits no logs by default', async () => {
		const compiler = new Compiler()
		compiler.compileSource(INLINE_SOURCE)
		await flushLogs()
		expect(capturedLog.length).toBe(0)
	})

	test('compiler suppresses info logs at warn level', async () => {
		const compiler = new Compiler({ loggingLevel: 'warn' })
		compiler.compileSource(INLINE_SOURCE)
		await flushLogs()
		expect(capturedLog.length).toBe(0)
	})

	test('compiler emits no logs at silent level', async () => {
		const compiler = new Compiler({ loggingLevel: 'silent' })
		compiler.compileSource(INLINE_SOURCE)
		await flushLogs()
		expect(capturedLog.length).toBe(0)
	})

	test('compiler logs context for filesystem errors', async () => {
		const compiler = new Compiler({ loggingLevel: 'info' })
		const missing = join(process.cwd(), 'libs', 'compiler', 'test', 'fixtures', 'missing.sol')
		let threw = false
		try {
			compiler.compileFiles([missing])
		} catch {
			threw = true
		}
		expect(threw).toBe(true)
		await flushLogs()
		const combined = [...capturedLog, ...capturedError]
		expect(combined.some((line) => line.includes('compiling filesystem sources'))).toBe(true)
		expect(
			combined.some(
				(line) => line.includes('failed to read source file') || line.includes('Failed to read source file'),
			),
		).toBe(true)
	})

	test('ast helper logs structural changes', async () => {
		const ast = new Ast({ loggingLevel: 'info' })
		ast.fromSource(INLINE_SOURCE)
		await flushLogs()
		expect(capturedLog.some((line) => line.includes('loading AST from source text'))).toBe(true)
	})

	test('contract helpers emit lifecycle logs', async () => {
		// Ensure the JS logger is initialised before creating contract instances.
		new Compiler({ loggingLevel: 'info' })
		await flushLogs()
		capturedLog.length = 0

		const contract = new Contract({ name: 'LoggingContract' })
		contract.withAddress('0x1234')
		await flushLogs()
		expect(capturedLog.some((line) => line.includes('contract address'))).toBe(true)
	})
})
