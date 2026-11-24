const assert = require('node:assert/strict')
const { mkdtempSync, realpathSync, rmSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { test } = require('node:test')
const { Ast, Compiler } = require('..')

test('loads the native package and recompiles an instrumented Solidity AST', () => {
	const ast = new Ast()
		.fromSource('contract Counter { uint256 private value; }')
		.injectShadow('function readValue() external view returns (uint256) { return value; }')
	const compiler = new Compiler({ cacheEnabled: false })
	const output = compiler.compileSources({ 'Counter.sol': ast.sourceUnit() })
	assert.equal(output.hasCompilerErrors(), false, JSON.stringify(output.diagnostics))
	const contract = output.artifacts['Counter.sol'].contracts.Counter
	assert.match(contract.deployedBytecode.hex, /^0x[\da-f]+$/)
	assert.ok(contract.abi.some((entry) => entry.type === 'function' && entry.name === 'readValue'))
})

test('resolves Solidity imports and returns the same native artifact from the project cache', (t) => {
	const root = realpathSync(mkdtempSync(join(tmpdir(), 'tevm-compiler-e2e-')))
	t.after(() => rmSync(root, { recursive: true, force: true }))
	const path = join(root, 'Counter.sol')
	writeFileSync(path, 'pragma solidity ^0.8.30; import "./Base.sol"; contract Counter is Base {}')
	writeFileSync(join(root, 'Base.sol'), 'pragma solidity ^0.8.30; contract Base { uint256 public value; }')

	const compiler = Compiler.fromRoot(root)
	const first = compiler.compileProject()
	assert.equal(first.hasCompilerErrors(), false, JSON.stringify(first.diagnostics))
	const contract = first.artifacts[path].contracts.Counter
	assert.match(contract.creationBytecode.hex, /^0x[\da-f]+$/)
	assert.ok(contract.abi.some((entry) => entry.type === 'function' && entry.name === 'value'))

	const cached = compiler.compileProject()
	assert.equal(cached.hasCompilerErrors(), false, JSON.stringify(cached.diagnostics))
	assert.deepEqual(cached.artifactsJson.contracts, {})
	const cachedSource = cached.artifacts[path]
	assert.equal(cachedSource.contracts.Counter.creationBytecode.hex, contract.creationBytecode.hex)
	assert.equal(cachedSource.solcVersion, first.artifacts[path].solcVersion)
	assert.deepEqual(cachedSource.ast.sourceUnit(), first.artifacts[path].ast.sourceUnit())
})
