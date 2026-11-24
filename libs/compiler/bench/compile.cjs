const assert = require('node:assert/strict')
const { mkdtempSync, readFileSync, rmSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { performance } = require('node:perf_hooks')
const { Compiler } = require('..')

const sources = {
	'Counter.sol': readFileSync(join(__dirname, '../test/fixtures/contracts/Counter.sol'), 'utf8'),
	'Greeter.sol': readFileSync(join(__dirname, '../test/fixtures/contracts/Greeter.sol'), 'utf8'),
}

function compile(compiler) {
	const output = compiler.compileSources(sources)
	assert.equal(output.hasCompilerErrors(), false, JSON.stringify(output.diagnostics))
	const contracts = Object.assign({}, ...Object.values(output.artifacts).map((artifact) => artifact.contracts))
	const bytecode = [contracts.Counter.deployedBytecode.hex, contracts.Greeter.deployedBytecode.hex]
	for (const hex of bytecode) assert.match(hex, /^0x[\da-f]+$/)
	return bytecode
}

function measure(label, compiler, iterations) {
	const expected = compile(compiler)
	const started = performance.now()
	for (let i = 0; i < iterations; i++) {
		assert.deepEqual(compile(compiler), expected)
	}
	const elapsed = performance.now() - started
	console.log(`${label}: ${(elapsed / iterations).toFixed(2)} ms/operation (${iterations} iterations)`)
}

const root = mkdtempSync(join(tmpdir(), 'tevm-compiler-bench-'))
try {
	const uncached = Compiler.fromRoot(root, { cacheEnabled: false })
	const cached = Compiler.fromRoot(root, { cacheEnabled: true })
	measure('Two Solidity sources, uncached', uncached, 10)
	measure('Two Solidity sources, warm cache', cached, 30)
} finally {
	rmSync(root, { recursive: true, force: true })
}
