import { describe, expect, it } from 'vitest'
import { Compiler } from './Compiler.js'
import type { FileAccessObject } from './resolutions/FileAccessObject.js'

describe('Compiler with FileAccessObject', () => {
	const simpleContract = `
    // SPDX-License-Identifier: MIT
    pragma solidity ^0.8.0;
    contract Simple {
      uint public value = 42;
    }
  `

	const virtualFs: Record<string, string> = {
		'Simple.sol': simpleContract,
	}

	const createVirtualFao = (): FileAccessObject => {
		// Helper to get filename from path (handles both relative and absolute paths)
		const getFilename = (path: string) => {
			const parts = path.replace(/\\/g, '/').split('/')
			return parts[parts.length - 1] || path
		}

		return {
			readFile: async (path) => {
				const filename = getFilename(path)
				return virtualFs[filename] || virtualFs[path] || ''
			},
			readFileSync: (path) => {
				const filename = getFilename(path)
				return virtualFs[filename] || virtualFs[path] || ''
			},
			exists: async (path) => {
				const filename = getFilename(path)
				return filename in virtualFs || path in virtualFs
			},
			existsSync: (path) => {
				const filename = getFilename(path)
				return filename in virtualFs || path in virtualFs
			},
		}
	}

	describe('factory-level FAO', () => {
		it('should use factory FAO for all compilations', async () => {
			const compiler = new Compiler({
				fileAccessObject: createVirtualFao(),
			})
			// @ts-expect-error - Accessing private property for testing
			compiler.solcInstance = require('solc')

			const result = await compiler.compileFiles(['Simple.sol'], {
				compilationOutput: ['abi'],
			})

			expect(result.compilationResult['Simple.sol']).toBeDefined()
		})
	})

	describe('per-compilation FAO override', () => {
		it('should allow overriding factory FAO', async () => {
			const factoryFao = createVirtualFao()
			const compiler = new Compiler({ fileAccessObject: factoryFao })
			// @ts-expect-error - Accessing private property for testing
			compiler.solcInstance = require('solc')

			// Override with different FAO
			const overrideFs: Record<string, string> = {
				'Override.sol': simpleContract.replace('Simple', 'Override'),
			}
			const overrideFao: FileAccessObject = {
				readFile: async (path) => overrideFs[path] || '',
				readFileSync: (path) => overrideFs[path] || '',
				exists: async (path) => path in overrideFs,
				existsSync: (path) => path in overrideFs,
			}

			const result = await compiler.compileFiles(['Override.sol'], {
				fileAccessObject: overrideFao,
				compilationOutput: ['abi'],
			})

			// Warnings are okay, but should not have actual errors
			const hasErrors = result.errors?.some((e) => e.severity === 'error')
			expect(hasErrors).toBeFalsy()
			expect(result.compilationResult['Override.sol']).toBeDefined()
		})
	})

	describe('sync methods', () => {
		it('should work with compileFilesSync', () => {
			const compiler = new Compiler({
				fileAccessObject: createVirtualFao(),
			})
			// @ts-expect-error - Accessing private property for testing
			compiler.solcInstance = require('solc')

			const result = compiler.compileFilesSync(['Simple.sol'], {
				compilationOutput: ['abi'],
			})

			expect(result.compilationResult['Simple.sol']).toBeDefined()
		})
	})

	describe('shadow compilation', () => {
		it('should work with compileFilesWithShadow', async () => {
			const compiler = new Compiler({
				fileAccessObject: createVirtualFao(),
			})
			// @ts-expect-error - Accessing private property for testing
			compiler.solcInstance = require('solc')

			const shadow = 'function testFunc() public pure returns (uint) { return 123; }'

			const result = await compiler.compileFilesWithShadow(['Simple.sol'], shadow, {
				sourceLanguage: 'Solidity',
				shadowLanguage: 'Solidity',
				injectIntoContractName: 'Simple',
				compilationOutput: ['abi'],
			})

			expect(result.compilationResult['Simple.sol']).toBeDefined()
		})
	})
})
