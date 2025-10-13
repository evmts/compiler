import { describe, expect, it, vi } from 'vitest'
import type { FileAccessObject } from '../resolutions/FileAccessObject.js'
import { compileFiles } from './compileFiles.js'

// Mock solc to avoid network requests
vi.mock('@tevm/solc', async () => {
	const actual = await vi.importActual<typeof import('@tevm/solc')>('@tevm/solc')
	return {
		...actual,
		createSolc: vi.fn().mockResolvedValue(require('solc')),
	}
})

describe('compileFiles with custom FileAccessObject', () => {
	const simpleContract = `
    // SPDX-License-Identifier: MIT
    pragma solidity ^0.8.0;

    contract SimpleStorage {
      uint256 value;

      function setValue(uint256 _value) public {
        value = _value;
      }

      function getValue() public view returns (uint256) {
        return value;
      }
    }
  `

	describe('virtual filesystem', () => {
		it('should compile from in-memory sources', async () => {
			const virtualFs: Record<string, string> = {
				'SimpleStorage.sol': simpleContract,
			}

			// Helper to extract filename from path
			const getFilename = (path: string) => {
				const parts = path.replace(/\\/g, '/').split('/')
				return parts[parts.length - 1] || path
			}

			const fao: FileAccessObject = {
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

			const result = await compileFiles(['SimpleStorage.sol'], {
				fileAccessObject: fao,
				compilationOutput: ['abi', 'bytecode'],
			})

			// Warnings are okay, but should not have actual errors
			const hasErrors = result.errors?.some((e) => e.severity === 'error')
			expect(hasErrors).toBeFalsy()
			expect(result.compilationResult).toBeDefined()
			expect(result.compilationResult['SimpleStorage.sol']).toBeDefined()
			expect(result.compilationResult['SimpleStorage.sol']?.contract['SimpleStorage']).toBeDefined()
		})

		it('should handle multiple files with imports', async () => {
			const virtualFs: Record<string, string> = {
				'Library.sol': `
          // SPDX-License-Identifier: MIT
          pragma solidity ^0.8.0;
          library Math {
            function add(uint a, uint b) internal pure returns (uint) {
              return a + b;
            }
          }
        `,
				'Consumer.sol': `
          // SPDX-License-Identifier: MIT
          pragma solidity ^0.8.0;
          import "./Library.sol";

          contract Consumer {
            function addNumbers(uint a, uint b) public pure returns (uint) {
              return Math.add(a, b);
            }
          }
        `,
			}

			const getFilename = (path: string) => {
				const parts = path.replace(/\\/g, '/').split('/')
				return parts[parts.length - 1] || path
			}

			const fao: FileAccessObject = {
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

			const result = await compileFiles(['Consumer.sol', 'Library.sol'], {
				fileAccessObject: fao,
				compilationOutput: ['abi'],
			})

			// Warnings are okay, but should not have actual errors
			const hasErrors = result.errors?.some((e) => e.severity === 'error')
			expect(hasErrors).toBeFalsy()
			expect(result.compilationResult['Consumer.sol']).toBeDefined()
			expect(result.compilationResult['Library.sol']).toBeDefined()
		})
	})

	describe('error handling', () => {
		it('should propagate FAO read errors', async () => {
			const fao: FileAccessObject = {
				readFile: async () => {
					throw new Error('Permission denied')
				},
				readFileSync: () => {
					throw new Error('Permission denied')
				},
				exists: async () => true,
				existsSync: () => true,
			}

			await expect(compileFiles(['contract.sol'], { fileAccessObject: fao })).rejects.toThrow('Failed to read file')
		})

		it('should handle non-existent files gracefully', async () => {
			const fao: FileAccessObject = {
				readFile: async (path) => {
					throw new Error(`ENOENT: no such file or directory, open '${path}'`)
				},
				readFileSync: (path) => {
					throw new Error(`ENOENT: no such file or directory, open '${path}'`)
				},
				exists: async () => false,
				existsSync: () => false,
			}

			await expect(compileFiles(['NonExistent.sol'], { fileAccessObject: fao })).rejects.toThrow()
		})
	})

	describe('instrumentation', () => {
		it('should allow logging file access', async () => {
			const accessLog: string[] = []

			const virtualFs: Record<string, string> = {
				'Test.sol': simpleContract,
			}

			const getFilename = (path: string) => {
				const parts = path.replace(/\\/g, '/').split('/')
				return parts[parts.length - 1] || path
			}

			const loggingFao: FileAccessObject = {
				readFile: async (path) => {
					accessLog.push(`readFile: ${path}`)
					const filename = getFilename(path)
					return virtualFs[filename] || virtualFs[path] || ''
				},
				readFileSync: (path) => {
					accessLog.push(`readFileSync: ${path}`)
					const filename = getFilename(path)
					return virtualFs[filename] || virtualFs[path] || ''
				},
				exists: async (path) => {
					accessLog.push(`exists: ${path}`)
					const filename = getFilename(path)
					return filename in virtualFs || path in virtualFs
				},
				existsSync: (path) => {
					accessLog.push(`existsSync: ${path}`)
					const filename = getFilename(path)
					return filename in virtualFs || path in virtualFs
				},
			}

			await compileFiles(['Test.sol'], {
				fileAccessObject: loggingFao,
				compilationOutput: ['abi'],
			})

			expect(accessLog.length).toBeGreaterThan(0)
			// Check that readFile was called with some path containing Test.sol
			expect(accessLog.some((log) => log.includes('readFile') && log.includes('Test.sol'))).toBe(true)
		})
	})

	describe('caching', () => {
		it('should allow caching file reads', async () => {
			const cache = new Map<string, string>()
			let readCount = 0

			const virtualFs: Record<string, string> = {
				'Cached.sol': simpleContract,
			}

			const getFilename = (path: string) => {
				const parts = path.replace(/\\/g, '/').split('/')
				return parts[parts.length - 1] || path
			}

			const cachingFao: FileAccessObject = {
				readFile: async (path) => {
					if (cache.has(path)) {
						return cache.get(path)!
					}
					readCount++
					const filename = getFilename(path)
					const content = virtualFs[filename] || virtualFs[path] || ''
					cache.set(path, content)
					return content
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

			// First compilation
			await compileFiles(['Cached.sol'], {
				fileAccessObject: cachingFao,
				compilationOutput: ['abi'],
			})
			expect(readCount).toBe(1)

			// Second compilation (should use cache)
			await compileFiles(['Cached.sol'], {
				fileAccessObject: cachingFao,
				compilationOutput: ['abi'],
			})
			expect(readCount).toBe(1) // Still 1, read from cache
		})
	})
})
